/// Connecteur Browser — contrôle Chrome via @playwright/mcp (Microsoft).
///
/// Utilise un subprocess `bunx @playwright/mcp --browser chrome` persistant.
/// Zéro téléchargement supplémentaire si Chrome est déjà installé (cas général).
/// Config : ~/.osmozzz/browser.toml
use serde_json::{json, Value};
use std::sync::OnceLock;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize, Clone)]
pub struct BrowserConfig {
    /// "chrome" (défaut) | "chromium" | "msedge"
    #[serde(default = "default_browser")]
    pub browser: String,
    /// Mode sans fenêtre — recommandé pour le daemon
    #[serde(default = "default_headless")]
    pub headless: bool,
}

fn default_browser() -> String { "chrome".to_string() }
fn default_headless() -> bool { true }

impl BrowserConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/browser.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }
}

// ─── Subprocess persistant ────────────────────────────────────────────────────

struct BrowserProcess {
    stdin:   ChildStdin,
    stdout:  BufReader<ChildStdout>,
    _child:  Child,
    next_id: u64,
}

static PROC: OnceLock<Mutex<Option<BrowserProcess>>> = OnceLock::new();

fn proc_mutex() -> &'static Mutex<Option<BrowserProcess>> {
    PROC.get_or_init(|| Mutex::new(None))
}

/// Démarre le subprocess @playwright/mcp si pas encore démarré.
async fn ensure_started(cfg: &BrowserConfig) -> Result<(), String> {
    let mut guard = proc_mutex().lock().await;
    if guard.is_some() { return Ok(()); }

    let bun = find_bun().ok_or("Bun non trouvé — relance le daemon OSMOzzz".to_string())?;

    let mut args = vec![
        "x".to_string(),
        "--bun".to_string(),
        "@playwright/mcp".to_string(),
        format!("--browser={}", cfg.browser),
    ];
    if cfg.headless { args.push("--headless".to_string()); }

    let mut child = tokio::process::Command::new(&bun)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Impossible de démarrer @playwright/mcp : {}", e))?;

    let stdin  = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());

    let mut proc = BrowserProcess { stdin, stdout, _child: child, next_id: 1 };

    // Handshake MCP
    rpc_call_inner(&mut proc, "initialize", json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": { "name": "osmozzz", "version": "1.0" }
    })).await.map_err(|e| format!("Handshake playwright/mcp échoué : {}", e))?;

    notify_inner(&mut proc, "notifications/initialized", json!({})).await
        .map_err(|e| format!("Notification initialized échouée : {}", e))?;

    *guard = Some(proc);
    Ok(())
}

async fn rpc_call_inner(proc: &mut BrowserProcess, method: &str, params: Value) -> Result<Value, String> {
    let id = proc.next_id;
    proc.next_id += 1;
    let req = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
    let mut line = serde_json::to_string(&req).unwrap();
    line.push('\n');
    proc.stdin.write_all(line.as_bytes()).await.map_err(|e| e.to_string())?;
    proc.stdin.flush().await.map_err(|e| e.to_string())?;

    // Lit les lignes jusqu'à trouver la réponse avec notre id
    loop {
        let mut buf = String::new();
        proc.stdout.read_line(&mut buf).await.map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(buf.trim()).map_err(|e| e.to_string())?;
        if v.get("id") == Some(&json!(id)) {
            if let Some(err) = v.get("error") {
                return Err(err.to_string());
            }
            return Ok(v["result"].clone());
        }
        // Notifications/autres messages → ignorer et continuer
    }
}

async fn notify_inner(proc: &mut BrowserProcess, method: &str, params: Value) -> Result<(), String> {
    let req = json!({ "jsonrpc": "2.0", "method": method, "params": params });
    let mut line = serde_json::to_string(&req).unwrap();
    line.push('\n');
    proc.stdin.write_all(line.as_bytes()).await.map_err(|e| e.to_string())?;
    proc.stdin.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Appelle un tool MCP sur le subprocess playwright.
async fn call_tool(tool: &str, args: Value, cfg: &BrowserConfig) -> Result<String, String> {
    ensure_started(cfg).await?;
    let mut guard = proc_mutex().lock().await;
    let proc = guard.as_mut().ok_or("Subprocess browser non démarré")?;

    let result = rpc_call_inner(proc, "tools/call", json!({
        "name": tool,
        "arguments": args,
    })).await;

    // Si le subprocess est mort, on le relance au prochain appel
    if result.is_err() {
        *guard = None;
    }

    let result = result?;

    // Extrait le texte du résultat MCP (format content: [{type: text, text: ...}])
    if let Some(content) = result.get("content") {
        if let Some(arr) = content.as_array() {
            let texts: Vec<&str> = arr.iter()
                .filter_map(|c| {
                    if c.get("type")?.as_str()? == "text" {
                        c.get("text")?.as_str()
                    } else {
                        None
                    }
                })
                .collect();
            if !texts.is_empty() {
                return Ok(texts.join("\n"));
            }
        }
    }
    Ok(serde_json::to_string(&result).unwrap_or_default())
}

fn find_bun() -> Option<String> {
    let candidates = [
        std::env::var("HOME").ok().map(|h| format!("{}/.bun/bin/bun", h)),
        Some("/usr/local/bin/bun".to_string()),
        Some("/opt/homebrew/bin/bun".to_string()),
    ];
    candidates.into_iter().flatten().find(|p| std::path::Path::new(p).exists())
}

// ─── Tools déclarés à Claude ──────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "browser_navigate",
            "description": "Navigue vers une URL dans le navigateur. Utiliser pour ouvrir une page web.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "URL complète à ouvrir (ex: https://google.com)" }
                },
                "required": ["url"]
            }
        }),
        json!({
            "name": "browser_snapshot",
            "description": "Retourne l'arbre d'accessibilité de la page courante (structure HTML lisible par l'IA). Utiliser pour comprendre le contenu d'une page avant de cliquer.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "browser_click",
            "description": "Clique sur un élément de la page via son sélecteur CSS ou son texte visible.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "element": { "type": "string", "description": "Description de l'élément ou sélecteur CSS" },
                    "ref": { "type": "string", "description": "Référence de l'élément depuis browser_snapshot" }
                },
                "required": ["element"]
            }
        }),
        json!({
            "name": "browser_type",
            "description": "Tape du texte dans un champ de saisie.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "element": { "type": "string", "description": "Description du champ ou sélecteur CSS" },
                    "ref": { "type": "string", "description": "Référence depuis browser_snapshot" },
                    "text": { "type": "string", "description": "Texte à taper" }
                },
                "required": ["element", "text"]
            }
        }),
        json!({
            "name": "browser_fill",
            "description": "Remplit un champ de formulaire avec une valeur (efface d'abord le contenu existant).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "element": { "type": "string", "description": "Description du champ" },
                    "ref": { "type": "string", "description": "Référence depuis browser_snapshot" },
                    "value": { "type": "string", "description": "Valeur à remplir" }
                },
                "required": ["element", "value"]
            }
        }),
        json!({
            "name": "browser_press_key",
            "description": "Appuie sur une touche du clavier (ex: Enter, Tab, Escape, ArrowDown).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Nom de la touche (ex: Enter, Tab, Escape)" }
                },
                "required": ["key"]
            }
        }),
        json!({
            "name": "browser_screenshot",
            "description": "Prend une capture d'écran de la page courante et retourne l'image en base64.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "browser_scroll",
            "description": "Fait défiler la page vers le haut ou le bas.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "direction": { "type": "string", "enum": ["up", "down"], "description": "Direction du scroll" },
                    "amount": { "type": "number", "description": "Nombre de pixels (défaut: 500)" }
                },
                "required": ["direction"]
            }
        }),
        json!({
            "name": "browser_go_back",
            "description": "Revient à la page précédente (équivalent bouton Retour).",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "browser_evaluate",
            "description": "Exécute du JavaScript dans la page et retourne le résultat. Utile pour extraire des données ou déclencher des actions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expression": { "type": "string", "description": "Code JavaScript à exécuter" }
                },
                "required": ["expression"]
            }
        }),
        json!({
            "name": "browser_close",
            "description": "Ferme le navigateur et libère les ressources.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "browser_select_option",
            "description": "Sélectionne une option dans un menu déroulant (<select>).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "element": { "type": "string", "description": "Description du champ select ou sélecteur CSS" },
                    "ref": { "type": "string", "description": "Référence depuis browser_snapshot" },
                    "values": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Valeur(s) à sélectionner"
                    }
                },
                "required": ["element", "values"]
            }
        }),
        json!({
            "name": "browser_wait_for",
            "description": "Attend qu'un texte ou élément apparaisse dans la page (utile pour les pages dynamiques/chargement AJAX).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Texte à attendre dans la page" },
                    "timeout": { "type": "number", "description": "Timeout en millisecondes (défaut: 5000)" }
                },
                "required": ["text"]
            }
        }),
        json!({
            "name": "browser_file_upload",
            "description": "Upload un fichier local dans un input file de la page.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "element": { "type": "string", "description": "Description du champ file ou sélecteur CSS" },
                    "ref": { "type": "string", "description": "Référence depuis browser_snapshot" },
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Chemins absolus des fichiers à uploader"
                    }
                },
                "required": ["element", "paths"]
            }
        }),
        json!({
            "name": "browser_new_tab",
            "description": "Ouvre un nouvel onglet dans le navigateur et navigue vers une URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "URL à ouvrir dans le nouvel onglet (optionnel)" }
                }
            }
        }),
        json!({
            "name": "browser_get_url",
            "description": "Retourne l'URL courante de la page active.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
    ]
}

// ─── Dispatcher ───────────────────────────────────────────────────────────────

pub async fn handle(tool: &str, args: &Value) -> Result<String, String> {
    let cfg = BrowserConfig::load().unwrap_or_else(|| BrowserConfig {
        browser: default_browser(),
        headless: default_headless(),
    });

    if tool == "browser_close" {
        let mut guard = proc_mutex().lock().await;
        *guard = None;
        return Ok("Navigateur fermé.".to_string());
    }

    call_tool(tool, args.clone(), &cfg).await
}
