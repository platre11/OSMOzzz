/// Implémentation manuelle du protocole MCP (Model Context Protocol) v2024-11-05.
/// Transport : stdin/stdout (JSON-RPC 2.0).
///
/// CRITIQUE : tout ce qui va sur stdout doit être JSON-RPC pur.
///            Les logs vont UNIQUEMENT sur stderr (eprintln! / tracing vers stderr).
///
/// Watcher intégré : au démarrage, une tâche tokio surveille ~/Desktop et ~/Documents
/// en temps réel (FSEvents) et indexe automatiquement tout nouveau fichier.
use std::io::{self, BufRead, Write};
use std::sync::Arc;

use anyhow::{Context, Result};
use osmozzz_core::Embedder;
use osmozzz_embedder::Vault;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::Config;
use shellexpand;

// ─── Types JSON-RPC ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct Request {
    #[allow(dead_code)]
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct Response {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

impl Response {
    fn ok(id: Value, result: Value) -> Self {
        Self { jsonrpc: "2.0", id, result: Some(result), error: None }
    }
    fn err(id: Value, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(json!({"code": code, "message": message})),
        }
    }
}

// ─── Définition des outils MCP ───────────────────────────────────────────────

fn tools_list() -> Value {
    json!([
        {
            "name": "search_memory",
            "description": "Recherche sémantique dans ta mémoire personnelle locale (historique Chrome, PDFs, code source, fichiers texte, logs, config). Renvoie les extraits les plus pertinents avec leur source. Tout est 100% local, rien ne sort du Mac.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "La requête de recherche en langage naturel"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats à retourner (défaut: 5)",
                        "default": 5,
                        "minimum": 1,
                        "maximum": 20
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "find_file",
            "description": "Localise un fichier sur le Mac par son nom, son extension ou son chemin partiel. Exemples: 'scene.gltf', '.blend files', 'error.log'. Utilise la mémoire locale indexée.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Nom, extension ou chemin partiel du fichier à trouver"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 5)",
                        "default": 5,
                        "minimum": 1,
                        "maximum": 20
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "fetch_content",
            "description": "Lit le contenu d'un fichier spécifique à la demande (texte, code, PDF). L'IA appelle cet outil uniquement quand elle a besoin du contenu d'un fichier précis. Limite 100 Ko pour les textes, 20 Mo pour les PDFs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Chemin absolu du fichier à lire (ex: /Users/platre11/Documents/rapport.pdf)"
                    }
                },
                "required": ["path"]
            }
        },
        {
            "name": "get_recent_files",
            "description": "Liste les fichiers récemment modifiés dans Desktop et Documents. Utile pour reprendre une tâche en cours ou voir ce qui a changé récemment.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hours": {
                        "type": "integer",
                        "description": "Fenêtre temporelle en heures (défaut: 24)",
                        "default": 24,
                        "minimum": 1,
                        "maximum": 168
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre max de fichiers à retourner (défaut: 20)",
                        "default": 20,
                        "minimum": 1,
                        "maximum": 100
                    }
                }
            }
        },
        {
            "name": "list_directory",
            "description": "Liste le contenu d'un dossier (nom, type, taille, date de modification).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Chemin du dossier à lister (ex: ~/Desktop, ~/Documents)"
                    }
                },
                "required": ["path"]
            }
        }
    ])
}

// ─── Envoi d'une réponse sur stdout (pur JSON) ────────────────────────────────

fn send(response: &Response) {
    let json = serde_json::to_string(response).unwrap_or_default();
    println!("{}", json);
    io::stdout().flush().ok();
}

// ─── Point d'entrée de la commande `osmozzz mcp` ─────────────────────────────

pub async fn run(cfg: Config) -> Result<()> {
    eprintln!("[OSMOzzz MCP] Démarrage du serveur MCP...");

    let vault = Arc::new(
        Vault::open(
            &cfg.model_path,
            &cfg.tokenizer_path,
            cfg.db_path.to_str().unwrap_or(".osmozzz/vault"),
        )
        .await
        .context("Impossible d'ouvrir le vault")?,
    );

    eprintln!("[OSMOzzz MCP] Vault chargé.");
    eprintln!("[OSMOzzz MCP] En attente de messages MCP sur stdin...");
    eprintln!("[OSMOzzz MCP] Conseil : lance 'osmozzz daemon' en parallèle pour l'indexation en temps réel.");

    let stdin = io::stdin();
    let mut initialized = false;

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        eprintln!("[OSMOzzz MCP] Reçu: {}", line);

        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[OSMOzzz MCP] Parse error: {}", e);
                send(&Response::err(
                    Value::Null,
                    -32700,
                    &format!("Parse error: {}", e),
                ));
                continue;
            }
        };

        let id = req.id.clone().unwrap_or(Value::Null);

        match req.method.as_str() {
            // ── Handshake initial ──────────────────────────────────────────
            "initialize" => {
                initialized = true;
                send(&Response::ok(id, json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "osmozzz",
                        "version": "0.2.0"
                    }
                })));
            }

            // ── Notification ───────────────────────────────────────────────
            "notifications/initialized" => {
                eprintln!("[OSMOzzz MCP] Client initialisé.");
            }

            // ── Liste des outils ───────────────────────────────────────────
            "tools/list" => {
                send(&Response::ok(id, json!({
                    "tools": tools_list()
                })));
            }

            // ── Appel d'un outil ───────────────────────────────────────────
            "tools/call" => {
                if !initialized {
                    send(&Response::err(id, -32002, "Server not initialized"));
                    continue;
                }

                let tool_name = req.params["name"].as_str().unwrap_or("");
                let args = &req.params["arguments"];

                match tool_name {
                    "search_memory" => {
                        let query = match args["query"].as_str() {
                            Some(q) => q.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: query"));
                                continue;
                            }
                        };
                        let limit = args["limit"].as_u64().unwrap_or(5) as usize;
                        let limit = limit.clamp(1, 20);

                        eprintln!("[OSMOzzz MCP] Recherche: \"{}\" (limit={})", query, limit);

                        match vault.search(&query, limit).await {
                            Ok(results) => {
                                let text = format_results(&query, &results);
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": text}]
                                })));
                            }
                            Err(e) => {
                                eprintln!("[OSMOzzz MCP] Search error: {}", e);
                                send(&Response::err(id, -32603, &e.to_string()));
                            }
                        }
                    }

                    "find_file" => {
                        let name = match args["name"].as_str() {
                            Some(n) => n.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: name"));
                                continue;
                            }
                        };
                        let limit = args["limit"].as_u64().unwrap_or(5) as usize;
                        let limit = limit.clamp(1, 20);

                        eprintln!("[OSMOzzz MCP] Recherche fichier: \"{}\"", name);

                        // find_file = recherche sémantique avec requête orientée nom de fichier
                        let query = format!("File: {} Path:", name);
                        match vault.search(&query, limit).await {
                            Ok(results) => {
                                let text = format_file_results(&name, &results);
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": text}]
                                })));
                            }
                            Err(e) => {
                                eprintln!("[OSMOzzz MCP] Find file error: {}", e);
                                send(&Response::err(id, -32603, &e.to_string()));
                            }
                        }
                    }

                    "fetch_content" => {
                        let path_str = match args["path"].as_str() {
                            Some(p) => p.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: path"));
                                continue;
                            }
                        };
                        let path = std::path::Path::new(&path_str);
                        let text = fetch_file_content(path);
                        send(&Response::ok(id, json!({
                            "content": [{"type": "text", "text": text}]
                        })));
                    }

                    "get_recent_files" => {
                        let hours = args["hours"].as_u64().unwrap_or(24);
                        let limit = args["limit"].as_u64().unwrap_or(20) as usize;
                        let text = get_recent_files(hours, limit);
                        send(&Response::ok(id, json!({
                            "content": [{"type": "text", "text": text}]
                        })));
                    }

                    "list_directory" => {
                        let path_str = match args["path"].as_str() {
                            Some(p) => p.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: path"));
                                continue;
                            }
                        };
                        let expanded = shellexpand::tilde(&path_str).to_string();
                        let text = list_directory(std::path::Path::new(&expanded));
                        send(&Response::ok(id, json!({
                            "content": [{"type": "text", "text": text}]
                        })));
                    }

                    other => {
                        send(&Response::err(
                            id,
                            -32601,
                            &format!("Unknown tool: {}", other),
                        ));
                    }
                }
            }

            // ── Ping ───────────────────────────────────────────────────────
            "ping" => {
                send(&Response::ok(id, json!({})));
            }

            // ── Méthode inconnue ───────────────────────────────────────────
            other => {
                eprintln!("[OSMOzzz MCP] Méthode inconnue: {}", other);
                send(&Response::err(
                    id,
                    -32601,
                    &format!("Method not found: {}", other),
                ));
            }
        }
    }

    eprintln!("[OSMOzzz MCP] Connexion fermée.");
    Ok(())
}

// ─── Formatage des résultats ──────────────────────────────────────────────────

fn format_results(query: &str, results: &[osmozzz_core::SearchResult]) -> String {
    if results.is_empty() {
        return format!("Aucun résultat trouvé pour : \"{}\"", query);
    }

    let mut out = format!(
        "Résultats de recherche dans ta mémoire locale pour : \"{}\"\n\n",
        query
    );

    for (i, r) in results.iter().enumerate() {
        let chunk_info = match (r.chunk_index, r.chunk_total) {
            (Some(idx), Some(tot)) if tot > 1 => format!(" [partie {}/{}]", idx + 1, tot),
            _ => String::new(),
        };

        out.push_str(&format!(
            "{}. [{}]{} — Score: {:.2}\n",
            i + 1,
            r.source.to_uppercase(),
            chunk_info,
            r.score
        ));

        if let Some(title) = &r.title {
            out.push_str(&format!("   Titre : {}\n", title));
        }

        out.push_str(&format!("   Source : {}\n", r.url));
        out.push_str(&format!("   Extrait : {}\n\n", r.content.replace('\n', " ")));
    }

    out
}

// ─── fetch_content ────────────────────────────────────────────────────────────

const MAX_TEXT_READ: usize = 100 * 1024;       // 100 KB pour les textes
const MAX_PDF_READ: u64    = 20 * 1024 * 1024; // 20 MB pour les PDFs

fn fetch_file_content(path: &std::path::Path) -> String {
    if !path.exists() {
        return format!("Erreur : fichier introuvable : {}", path.display());
    }

    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if ext == "pdf" {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size > MAX_PDF_READ {
            return format!(
                "PDF trop volumineux ({} Mo) pour être lu en ligne. Chemin : {}",
                size / 1024 / 1024,
                path.display()
            );
        }
        return match pdf_extract::extract_text(path) {
            Ok(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    format!("PDF sans texte extractible (scanné ou protégé) : {}", path.display())
                } else if trimmed.len() > MAX_TEXT_READ {
                    format!("{}\n\n[... contenu tronqué à 100 Ko]", &trimmed[..MAX_TEXT_READ])
                } else {
                    trimmed.to_string()
                }
            }
            Err(e) => format!("Erreur lecture PDF : {}", e),
        };
    }

    // Fichier texte
    match std::fs::read_to_string(path) {
        Ok(content) => {
            if content.len() > MAX_TEXT_READ {
                format!("{}\n\n[... contenu tronqué à 100 Ko]", &content[..MAX_TEXT_READ])
            } else {
                content
            }
        }
        Err(_) => {
            // Fichier binaire
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            format!(
                "Fichier binaire non lisible.\nNom : {}\nChemin : {}\nTaille : {} Ko",
                path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                path.display(),
                size / 1024
            )
        }
    }
}

// ─── get_recent_files ─────────────────────────────────────────────────────────

fn get_recent_files(hours: u64, limit: usize) -> String {
    use std::time::{Duration, SystemTime};

    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(hours * 3600))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let home = match dirs_next::home_dir() {
        Some(h) => h,
        None => return "Impossible de localiser le home directory.".to_string(),
    };

    let watch_dirs = [home.join("Desktop"), home.join("Documents")];
    let mut entries: Vec<(SystemTime, std::path::PathBuf)> = Vec::new();

    for dir in &watch_dirs {
        if !dir.exists() { continue; }
        collect_recent(dir, &cutoff, &mut entries, 0);
    }

    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries.truncate(limit);

    if entries.is_empty() {
        return format!("Aucun fichier modifié dans les {} dernières heures.", hours);
    }

    let mut out = format!("Fichiers modifiés dans les {} dernières heures :\n\n", hours);
    for (ts, path) in &entries {
        let ago = SystemTime::now().duration_since(*ts)
            .map(|d| format!("il y a {}min", d.as_secs() / 60))
            .unwrap_or_else(|_| "?".to_string());
        out.push_str(&format!("• {} ({})\n", path.display(), ago));
    }
    out
}

fn collect_recent(
    dir: &std::path::Path,
    cutoff: &std::time::SystemTime,
    out: &mut Vec<(std::time::SystemTime, std::path::PathBuf)>,
    depth: usize,
) {
    if depth > 3 { return; }
    let rd = match std::fs::read_dir(dir) { Ok(r) => r, Err(_) => return };
    for entry in rd.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') { continue; }
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                if modified >= *cutoff {
                    out.push((modified, path.clone()));
                }
            }
            if meta.is_dir() && depth < 3 {
                collect_recent(&path, cutoff, out, depth + 1);
            }
        }
    }
}

// ─── list_directory ───────────────────────────────────────────────────────────

fn list_directory(path: &std::path::Path) -> String {
    if !path.exists() {
        return format!("Dossier introuvable : {}", path.display());
    }
    if !path.is_dir() {
        return format!("Ce chemin n'est pas un dossier : {}", path.display());
    }

    let rd = match std::fs::read_dir(path) {
        Ok(r) => r,
        Err(e) => return format!("Erreur lecture dossier : {}", e),
    };

    let mut entries: Vec<String> = Vec::new();
    for entry in rd.flatten() {
        let p = entry.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string();
        if name.starts_with('.') { continue; }
        if let Ok(meta) = entry.metadata() {
            let kind = if meta.is_dir() { "📁" } else { "📄" };
            let size = if meta.is_file() {
                format!(" ({} Ko)", meta.len() / 1024)
            } else {
                String::new()
            };
            entries.push(format!("{} {}{}", kind, name, size));
        }
    }

    if entries.is_empty() {
        return format!("Dossier vide : {}", path.display());
    }

    entries.sort();
    let mut out = format!("Contenu de {} :\n\n", path.display());
    for e in &entries {
        out.push_str(&format!("{}\n", e));
    }
    out
}

fn format_file_results(name: &str, results: &[osmozzz_core::SearchResult]) -> String {
    if results.is_empty() {
        return format!("Aucun fichier trouvé correspondant à : \"{}\"", name);
    }

    let mut out = format!("Fichiers trouvés pour : \"{}\"\n\n", name);

    for (i, r) in results.iter().enumerate() {
        // Extraire le chemin depuis l'URL (file:///path)
        let path = r.url.trim_start_matches("file://");
        // Enlever les ancres de chunk
        let path = path.split('#').next().unwrap_or(path);

        out.push_str(&format!(
            "{}. {}\n   Chemin : {}\n   Score : {:.2}\n\n",
            i + 1,
            r.title.as_deref().unwrap_or("Fichier"),
            path,
            r.score
        ));
    }

    out
}
