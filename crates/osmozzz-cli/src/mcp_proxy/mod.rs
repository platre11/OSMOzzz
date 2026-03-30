/// MCP Proxy — moteur générique pour les subprocessus MCP tiers.
/// Chaque connecteur (Jira, GitHub, Slack...) est dans son propre fichier.
///
/// Architecture :
///   osmozzz mcp (Rust) ──stdin/stdout──► Claude
///                       ──pipes──────► bunx @pkg/mcp-server (subprocess)
pub mod cloudflare;
pub mod github;
pub mod gitlab;
pub mod jira;
pub mod linear;
pub mod notion;
pub mod sentry;
pub mod slack;
pub mod supabase;

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

static REQ_ID: AtomicU64 = AtomicU64::new(100);

/// Timeout pour les appels aux subprocessus MCP (ex: Supabase API).
/// Au-delà, on retourne une erreur plutôt que de bloquer indéfiniment.
const TOOL_CALL_TIMEOUT_SECS: u64 = 60;

fn next_id() -> u64 {
    REQ_ID.fetch_add(1, Ordering::Relaxed)
}

// ─── Subprocess MCP générique ─────────────────────────────────────────────────

pub struct McpSubprocess {
    stdin:    std::process::ChildStdin,
    receiver: mpsc::Receiver<Value>,   // thread dédié qui lit stdout sans bloquer
    _child:   std::process::Child,
    pub tools: Vec<Value>,
    pub name:  String,
}

impl McpSubprocess {
    // ── Bun ──────────────────────────────────────────────────────────────────

    /// Cherche bun dans PATH ou ~/.bun/bin/
    pub fn find_bun() -> Option<String> {
        if std::process::Command::new("bun")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some("bun".to_string());
        }
        if let Some(home) = dirs_next::home_dir() {
            let path = home.join(".bun/bin/bun");
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }
        None
    }

    /// Installe Bun automatiquement via le script officiel
    pub fn install_bun() -> bool {
        eprintln!("[OSMOzzz MCP] Bun non trouvé — installation automatique...");
        let status = std::process::Command::new("bash")
            .args(["-c", "curl -fsSL https://bun.sh/install | bash"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        match status {
            Ok(s) if s.success() => {
                eprintln!("[OSMOzzz MCP] Bun installé avec succès.");
                true
            }
            _ => {
                eprintln!("[OSMOzzz MCP] Échec installation Bun.");
                false
            }
        }
    }

    /// Vérifie ou installe Bun, retourne le chemin
    pub fn ensure_bun() -> Option<String> {
        if let Some(p) = Self::find_bun() { return Some(p); }
        if Self::install_bun() { Self::find_bun() } else { None }
    }

    // ── Démarrage générique ───────────────────────────────────────────────────

    /// Démarre un subprocess MCP via bunx avec les env vars données
    pub fn start(
        name: &str,
        package: &str,
        env_vars: &[(&str, &str)],
    ) -> Option<Self> {
        Self::start_with_args(name, package, env_vars, &[])
    }

    pub fn start_with_args(
        name: &str,
        package: &str,
        env_vars: &[(&str, &str)],
        extra_args: &[&str],
    ) -> Option<Self> {
        let bun = Self::ensure_bun()?;

        eprintln!("[OSMOzzz MCP] Démarrage du subprocess {name} ({package})...");

        let mut cmd = std::process::Command::new(&bun);
        cmd.args(["x", "--bun", package])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        for arg in extra_args {
            cmd.arg(arg);
        }

        for (key, val) in env_vars {
            cmd.env(key, val);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| eprintln!("[OSMOzzz MCP] Spawn {name} échoué: {e}"))
            .ok()?;

        let stdin  = child.stdin.take()?;
        let stdout = child.stdout.take()?;

        // Thread dédié qui lit stdout du subprocess et envoie chaque ligne
        // JSON parsée dans un channel — élimine le blocage indéfini du main thread.
        let (tx, rx) = mpsc::channel::<Value>();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    if tx.send(v).is_err() { break; }
                }
            }
        });

        let mut sub = Self {
            stdin,
            receiver: rx,
            _child: child,
            tools: vec![],
            name: name.to_string(),
        };

        sub.initialize()?;
        sub.tools = sub.discover_tools().unwrap_or_default();

        eprintln!(
            "[OSMOzzz MCP] {} prêt — {} tools : {}",
            name,
            sub.tools.len(),
            sub.tools.iter()
                .filter_map(|t| t.get("name")?.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

        Some(sub)
    }

    // ── JSON-RPC ─────────────────────────────────────────────────────────────

    fn write_json(&mut self, value: &Value) -> Option<()> {
        let mut line = serde_json::to_string(value).ok()?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).ok()?;
        self.stdin.flush().ok()
    }

    fn read_response(&mut self, expected_id: u64) -> Option<Value> {
        let timeout  = Duration::from_secs(TOOL_CALL_TIMEOUT_SECS);
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                eprintln!(
                    "[OSMOzzz MCP] Timeout {} — pas de réponse après {}s",
                    self.name, TOOL_CALL_TIMEOUT_SECS
                );
                return None;
            }
            match self.receiver.recv_timeout(remaining) {
                Ok(v) => {
                    if v.get("id").and_then(|i| i.as_u64()) == Some(expected_id) {
                        return Some(v);
                    }
                    // notification ou message non-correspondant — continuer
                }
                Err(_) => return None, // timeout ou channel fermé
            }
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Option<Value> {
        let id = next_id();
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        self.write_json(&req)?;
        self.read_response(id)
    }

    fn initialize(&mut self) -> Option<()> {
        self.request("initialize", json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "osmozzz", "version": "1.0.0" }
        }))?;
        self.write_json(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
    }

    fn discover_tools(&mut self) -> Option<Vec<Value>> {
        let resp = self.request("tools/list", json!({}))?;
        resp.get("result")?
            .get("tools")?
            .as_array()
            .cloned()
    }

    // ── API publique ─────────────────────────────────────────────────────────

    pub fn call_tool(&mut self, tool_name: &str, arguments: &Value) -> Result<String, String> {
        let resp = self.request("tools/call", json!({
            "name": tool_name,
            "arguments": arguments
        })).ok_or_else(|| format!("Subprocess {} indisponible", self.name))?;

        if let Some(err) = resp.get("error") {
            return Err(format!("Erreur {} MCP: {}", self.name, err));
        }

        let text = resp
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("Action exécutée.");

        Ok(text.to_string())
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tools
            .iter()
            .filter_map(|t| t.get("name")?.as_str().map(String::from))
            .collect()
    }
}

// ─── Proxy paresseux (lazy) — subprocess démarré à la première utilisation ───

pub struct LazyProxy {
    pub name:    String,
    package:     String,
    env_vars:    Vec<(String, String)>,
    extra_args:  Vec<String>,
    subprocess:  Option<McpSubprocess>,
}

impl LazyProxy {
    pub fn new(name: &str, package: &str, env_vars: Vec<(String, String)>) -> Self {
        Self {
            name:       name.to_string(),
            package:    package.to_string(),
            env_vars,
            extra_args: vec![],
            subprocess: None,
        }
    }

    pub fn new_with_args(name: &str, package: &str, env_vars: Vec<(String, String)>, extra_args: Vec<String>) -> Self {
        Self {
            name:       name.to_string(),
            package:    package.to_string(),
            env_vars,
            extra_args,
            subprocess: None,
        }
    }

    fn ensure_started(&mut self) -> Option<&mut McpSubprocess> {
        if self.subprocess.is_none() {
            let env_refs: Vec<(&str, &str)> = self.env_vars.iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let arg_refs: Vec<&str> = self.extra_args.iter()
                .map(|s| s.as_str())
                .collect();
            self.subprocess = McpSubprocess::start_with_args(&self.name, &self.package, &env_refs, &arg_refs);
        }
        self.subprocess.as_mut()
    }

    /// Démarre le subprocess si nécessaire et retourne la liste des tools disponibles.
    pub fn list_tools(&mut self) -> Vec<Value> {
        self.ensure_started()
            .map(|p| p.tools.clone())
            .unwrap_or_default()
    }

    /// Démarre le subprocess si nécessaire et appelle le tool demandé.
    pub fn call_tool(&mut self, tool_name: &str, arguments: &Value) -> Result<String, String> {
        let name = self.name.clone();
        self.ensure_started()
            .ok_or_else(|| format!("Impossible de démarrer le subprocess {}", name))?
            .call_tool(tool_name, arguments)
    }
}

// ─── Charge tous les proxies configurés (sans démarrer les subprocessus) ─────

pub fn start_all_proxies() -> Vec<LazyProxy> {
    let mut proxies = Vec::new();

    if let Some(p) = jira::lazy()        { proxies.push(p); }
    if let Some(p) = github::lazy()      { proxies.push(p); }
    if let Some(p) = gitlab::lazy()      { proxies.push(p); }
    if let Some(p) = notion::lazy()      { proxies.push(p); }
    if let Some(p) = sentry::lazy()      { proxies.push(p); }
    if let Some(p) = cloudflare::lazy()  { proxies.push(p); }
    if let Some(p) = slack::lazy()       { proxies.push(p); }
    if let Some(p) = linear::lazy()      { proxies.push(p); }
    if let Some(p) = supabase::lazy()    { proxies.push(p); }

    proxies
}
