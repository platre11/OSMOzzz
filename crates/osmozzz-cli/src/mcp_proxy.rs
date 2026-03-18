/// MCP Proxy — gère les subprocessus MCP tiers (Jira, Slack, Linear...)
/// via Bun (runtime JS léger, remplace Node.js).
///
/// Architecture :
///   osmozzz mcp (Rust) ──stdin/stdout──► Claude
///                       ──pipes──────► bunx @pkg/mcp-server (subprocess)
///
/// Claude dirige → OSMOzzz exécute via le subprocess → résultat retourné à Claude.
/// Les données ne quittent jamais la machine.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicU64, Ordering};

static REQ_ID: AtomicU64 = AtomicU64::new(100);

fn next_id() -> u64 {
    REQ_ID.fetch_add(1, Ordering::Relaxed)
}

// ─── Config Jira ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct JiraConfig {
    pub base_url: String,
    pub email: String,
    pub token: String,
}

impl JiraConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/jira.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let table: toml::Value = content.parse().ok()?;
        Some(Self {
            base_url: table.get("base_url")?.as_str()?.to_string(),
            email:    table.get("email")?.as_str()?.to_string(),
            token:    table.get("token")?.as_str()?.to_string(),
        })
    }
}

// ─── Subprocess MCP ──────────────────────────────────────────────────────────

pub struct McpSubprocess {
    stdin:   std::process::ChildStdin,
    reader:  BufReader<std::process::ChildStdout>,
    _child:  std::process::Child,
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
        // Emplacement par défaut du script d'installation Bun
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
                eprintln!("[OSMOzzz MCP] Échec installation Bun — actions Jira indisponibles.");
                false
            }
        }
    }

    /// Vérifie ou installe Bun, retourne le chemin
    pub fn ensure_bun() -> Option<String> {
        if let Some(p) = Self::find_bun() { return Some(p); }
        if Self::install_bun() { Self::find_bun() } else { None }
    }

    // ── Démarrage Jira ───────────────────────────────────────────────────────

    /// Démarre le serveur MCP Jira (@aashari/mcp-server-atlassian-jira) via Bun
    pub fn start_jira(config: &JiraConfig) -> Option<Self> {
        let bun = Self::ensure_bun()?;

        eprintln!("[OSMOzzz MCP] Démarrage du serveur Jira MCP via Bun...");

        // Extrait le site name depuis l'URL (ex: "https://foo.atlassian.net" → "foo")
        let site_name = config.base_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('.')
            .next()
            .unwrap_or(&config.base_url)
            .to_string();

        let mut child = std::process::Command::new(&bun)
            .args(["x", "--bun", "@aashari/mcp-server-atlassian-jira"])
            .env("ATLASSIAN_SITE_NAME",  &site_name)
            .env("ATLASSIAN_USER_EMAIL", &config.email)
            .env("ATLASSIAN_API_TOKEN",  &config.token)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| eprintln!("[OSMOzzz MCP] Spawn Jira MCP échoué: {e}"))
            .ok()?;

        let stdin  = child.stdin.take()?;
        let stdout = child.stdout.take()?;
        let reader = BufReader::new(stdout);

        let mut sub = Self {
            stdin,
            reader,
            _child: child,
            tools: vec![],
            name: "jira".to_string(),
        };

        sub.initialize()?;
        sub.tools = sub.discover_tools().unwrap_or_default();

        eprintln!(
            "[OSMOzzz MCP] Jira MCP prêt — {} tools disponibles : {}",
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

    /// Lit les lignes stdout jusqu'à trouver la réponse avec l'ID attendu.
    /// Ignore les notifications (pas d'ID ou ID différent).
    fn read_response(&mut self, expected_id: u64) -> Option<Value> {
        for _ in 0..100 {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) | Err(_) => return None,
                Ok(_) => {}
            }
            let line = line.trim();
            if line.is_empty() { continue; }
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                if v.get("id").and_then(|i| i.as_u64()) == Some(expected_id) {
                    return Some(v);
                }
            }
        }
        None
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
        // Notification sans réponse attendue
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

    /// Appelle un tool du subprocess et retourne le texte résultat
    pub fn call_tool(&mut self, tool_name: &str, arguments: &Value) -> Result<String, String> {
        let resp = self.request("tools/call", json!({
            "name": tool_name,
            "arguments": arguments
        })).ok_or_else(|| "Subprocess Jira indisponible".to_string())?;

        if let Some(err) = resp.get("error") {
            return Err(format!("Erreur Jira MCP: {}", err));
        }

        // Extrait le texte depuis result.content[0].text
        let text = resp
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("Action Jira exécutée.");

        Ok(text.to_string())
    }

    /// Retourne les noms de tous les tools exposés par ce subprocess
    pub fn tool_names(&self) -> Vec<String> {
        self.tools
            .iter()
            .filter_map(|t| t.get("name")?.as_str().map(String::from))
            .collect()
    }
}
