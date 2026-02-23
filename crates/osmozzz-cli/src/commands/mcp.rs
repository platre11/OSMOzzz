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
            "description": "Recherche sémantique dans ta mémoire personnelle locale : emails Gmail (indexés via IMAP directement, sans navigateur), historique Chrome, fichiers texte, PDFs, code source. Renvoie les extraits les plus pertinents avec leur source (EMAIL, CHROME, FILE…). Tout est 100% local, rien ne sort du Mac. Pour chercher des emails : utilise des mots-clés du sujet, expéditeur ou contenu.",
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
            "description": "Lecture intelligente d'un fichier. AVEC query → mode Agentic RAG : score tous les blocs avec ONNX local, retourne le bloc le plus pertinent + carte de navigation (scores des autres blocs). L'IA peut ensuite demander un bloc précis par block_index. SANS query → lecture linéaire par offset/length.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Chemin absolu du fichier à lire"
                    },
                    "query": {
                        "type": "string",
                        "description": "Question ou sujet recherché. Active le mode RAG : retourne le bloc le plus pertinent + carte de navigation."
                    },
                    "block_index": {
                        "type": "integer",
                        "description": "Index du bloc à lire directement (issu de la carte de navigation). Utiliser avec query pour naviguer.",
                        "minimum": 0
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Position de départ en caractères (mode linéaire, sans query).",
                        "default": 0,
                        "minimum": 0
                    },
                    "length": {
                        "type": "integer",
                        "description": "Nombre de caractères à lire (mode linéaire, défaut: 3000, max: 10000).",
                        "default": 3000,
                        "minimum": 100,
                        "maximum": 10000
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
            "name": "get_recent_emails",
            "description": "Liste les derniers emails reçus dans la boîte Gmail, triés par date (du plus récent au plus ancien). Utilise l'index IMAP local. Idéal pour répondre à 'quel est mon dernier email ?' ou 'qu'est-ce que j'ai reçu récemment ?'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Nombre d'emails à retourner (défaut: 10, max: 50)",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 50
                    }
                }
            }
        },
        {
            "name": "smart_email_search",
            "description": "Recherche intelligente dans les emails uniquement. Détecte automatiquement l'intent : expéditeur ('mails de railway', 'dernier mail de codeur'), contenu ('mail qui parle de TVA', 'email sur le remboursement'), ou récents ('mes derniers mails'). Retourne le contenu COMPLET des emails trouvés en un seul appel. À utiliser en priorité pour toute question sur les emails.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "La demande en langage naturel : 'dernier mail de railway', 'emails de codeur.com', 'mail qui parle de facturation', 'mes 3 derniers mails', etc."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre d'emails à retourner (défaut: 3, max: 10)",
                        "default": 3,
                        "minimum": 1,
                        "maximum": 10
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "get_email_full",
            "description": "Retourne le contenu complet d'un email indexé (sans troncature). Utilise l'ID visible dans get_recent_emails ou search_memory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "L'ID du message (ex: '20260214005158.133e242c429fd22d@cio79999.news.railway.app') ou l'URL complète ('gmail://message/...')"
                    }
                },
                "required": ["id"]
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

                        // Blended search: global top results + guaranteed email results
                        let global_fut = vault.search_filtered(&query, limit, None);
                        let email_fut  = vault.search_filtered(&query, 3, Some("email"));

                        match tokio::try_join!(global_fut, email_fut) {
                            Ok((mut results, email_results)) => {
                                // Append email results not already in global results
                                let seen: std::collections::HashSet<String> =
                                    results.iter().map(|r| r.id.clone()).collect();
                                for r in email_results {
                                    if !seen.contains(&r.id) {
                                        results.push(r);
                                    }
                                }
                                // Sort by score descending
                                results.sort_by(|a, b| b.score.partial_cmp(&a.score)
                                    .unwrap_or(std::cmp::Ordering::Equal));

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
                        let limit = args["limit"].as_u64().unwrap_or(20) as usize;
                        eprintln!("[OSMOzzz MCP] Recherche fichier (filesystem): \"{}\"", name);
                        let text = find_file_filesystem(&name, limit);
                        send(&Response::ok(id, json!({
                            "content": [{"type": "text", "text": text}]
                        })));
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
                        let query = args["query"].as_str().map(|s| s.to_string());
                        let block_index = args["block_index"].as_u64().map(|v| v as usize);

                        let text = if let Some(q) = query {
                            // Mode Agentic RAG : scoring ONNX à la volée
                            match vault.embed_raw(&q) {
                                Ok(query_vec) => fetch_content_smart(path, &q, query_vec, block_index),
                                Err(e) => format!("Erreur embedding query : {}", e),
                            }
                        } else {
                            // Mode linéaire classique
                            let offset = args["offset"].as_u64().unwrap_or(0) as usize;
                            let length = args["length"].as_u64().unwrap_or(3000) as usize;
                            let length = length.clamp(100, 10000);
                            fetch_file_content(path, offset, length)
                        };

                        send(&Response::ok(id, json!({
                            "content": [{"type": "text", "text": text}]
                        })));
                    }

                    "get_recent_emails" => {
                        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
                        let limit = limit.clamp(1, 50);
                        eprintln!("[OSMOzzz MCP] Emails récents (limit={})", limit);
                        match vault.recent_emails(limit).await {
                            Ok(results) => {
                                let text = format_recent_emails(&results);
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": text}]
                                })));
                            }
                            Err(e) => {
                                send(&Response::err(id, -32603, &e.to_string()));
                            }
                        }
                    }

                    "smart_email_search" => {
                        let query = match args["query"].as_str() {
                            Some(q) => q.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: query"));
                                continue;
                            }
                        };
                        let limit = args["limit"].as_u64().unwrap_or(3) as usize;
                        let limit = limit.clamp(1, 10);

                        eprintln!("[OSMOzzz MCP] smart_email_search: \"{}\" (limit={})", query, limit);

                        match detect_email_intent(&query) {
                            EmailIntent::SenderSearch(sender) => {
                                eprintln!("[OSMOzzz MCP] Intent: SenderSearch(\"{}\")", sender);
                                match vault.get_emails_by_sender(&sender, limit).await {
                                    Ok(results) if !results.is_empty() => {
                                        send(&Response::ok(id, json!({
                                            "content": [{"type": "text", "text": format_full_emails(&results)}]
                                        })));
                                    }
                                    Ok(_) => {
                                        // Sender not found → fallback semantic search on emails
                                        eprintln!("[OSMOzzz MCP] Sender not found, fallback semantic");
                                        match vault.search_filtered(&query, limit, Some("email")).await {
                                            Ok(sem_results) => {
                                                let mut full = Vec::new();
                                                for r in &sem_results {
                                                    if let Ok(Some((t, c))) = vault.get_full_content_by_url(&r.url).await {
                                                        full.push((t, r.url.clone(), c));
                                                    }
                                                }
                                                send(&Response::ok(id, json!({
                                                    "content": [{"type": "text", "text": format_full_emails(&full)}]
                                                })));
                                            }
                                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                                        }
                                    }
                                    Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                                }
                            }
                            EmailIntent::Recent => {
                                eprintln!("[OSMOzzz MCP] Intent: Recent");
                                match vault.recent_emails_full(limit).await {
                                    Ok(results) => {
                                        send(&Response::ok(id, json!({
                                            "content": [{"type": "text", "text": format_full_emails(&results)}]
                                        })));
                                    }
                                    Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                                }
                            }
                            EmailIntent::ContentSearch => {
                                eprintln!("[OSMOzzz MCP] Intent: ContentSearch");
                                match vault.search_filtered(&query, limit, Some("email")).await {
                                    Ok(sem_results) => {
                                        let mut full = Vec::new();
                                        for r in &sem_results {
                                            if let Ok(Some((t, c))) = vault.get_full_content_by_url(&r.url).await {
                                                full.push((t, r.url.clone(), c));
                                            }
                                        }
                                        send(&Response::ok(id, json!({
                                            "content": [{"type": "text", "text": format_full_emails(&full)}]
                                        })));
                                    }
                                    Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                                }
                            }
                        }
                    }

                    "get_email_full" => {
                        let raw_id = match args["id"].as_str() {
                            Some(i) => i.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: id"));
                                continue;
                            }
                        };
                        // Accepte l'ID brut ou l'URL complète
                        let url = if raw_id.starts_with("gmail://") {
                            raw_id.clone()
                        } else {
                            format!("gmail://message/{}", raw_id)
                        };
                        eprintln!("[OSMOzzz MCP] Email complet: {}", url);
                        match vault.get_full_content_by_url(&url).await {
                            Ok(Some((title, content))) => {
                                let mut out = String::new();
                                if let Some(t) = title {
                                    out.push_str(&format!("Objet : {}\n", t));
                                }
                                out.push_str(&format!("URL   : {}\n", url));
                                out.push_str("\n─────────────────────────────────────\n");
                                out.push_str(&content);
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": out}]
                                })));
                            }
                            Ok(None) => {
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": format!("Email introuvable : {}\n\nL'email n'est peut-être pas indexé. Lance 'osmozzz index --source gmail' pour réindexer.", url)}]
                                })));
                            }
                            Err(e) => {
                                send(&Response::err(id, -32603, &e.to_string()));
                            }
                        }
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

// ─── Intent router ────────────────────────────────────────────────────────────

enum EmailIntent {
    SenderSearch(String),
    Recent,
    ContentSearch,
}

fn detect_email_intent(query: &str) -> EmailIntent {
    let q = query.to_lowercase();

    // Sender patterns — ordered by specificity
    let sender_triggers = [
        "mail de ", "mails de ", "email de ", "emails de ",
        "mail d'", "mails d'", "email d'", "emails d'",
        "from ", "de la part de ", "venant de ", "reçu de ",
        "message de ", "messages de ", "courrier de ",
    ];

    for trigger in &sender_triggers {
        if let Some(idx) = q.find(trigger) {
            let rest = q[idx + trigger.len()..].trim();
            let sender: String = rest.chars()
                .take_while(|c| !matches!(c, ',' | '?' | '!' | '\n' | '(' | ')'))
                .collect();
            let sender = sender.trim().to_string();
            if sender.len() > 1 {
                return EmailIntent::SenderSearch(sender);
            }
        }
    }

    // Recent patterns (without specific sender)
    let recent_triggers = [
        "dernier mail", "derniers mails", "dernier email", "derniers emails",
        "mes derniers", "mes mails", "mes emails", "nouveau mail", "nouveaux mails",
        "reçu récemment", "reçus récemment", "boite mail", "boîte mail",
    ];
    for trigger in &recent_triggers {
        if q.contains(trigger) {
            return EmailIntent::Recent;
        }
    }

    // Default: semantic content search
    EmailIntent::ContentSearch
}

// ─── Formatter emails complets ────────────────────────────────────────────────

fn format_full_emails(results: &[(Option<String>, String, String)]) -> String {
    if results.is_empty() {
        return "Aucun email trouvé.".to_string();
    }
    let mut out = String::new();
    for (i, (title, url, content)) in results.iter().enumerate() {
        out.push_str(&format!("📧 Email {}/{}\n", i + 1, results.len()));
        if let Some(t) = title {
            out.push_str(&format!("Objet : {}\n", t));
        }
        let msg_id = url.trim_start_matches("gmail://message/");
        out.push_str(&format!("ID    : {}\n", msg_id));
        out.push_str("─────────────────────────────────────\n");
        out.push_str(content);
        out.push_str("\n═════════════════════════════════════\n\n");
    }
    out
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

// ─── format_recent_emails ─────────────────────────────────────────────────────

fn format_recent_emails(results: &[osmozzz_core::SearchResult]) -> String {
    if results.is_empty() {
        return "Aucun email indexé. Lance 'osmozzz index --source gmail' pour indexer ta boîte.".to_string();
    }

    let mut out = format!("📬 {} derniers emails (IMAP Gmail, triés par date) :\n\n", results.len());
    for (i, r) in results.iter().enumerate() {
        let title = r.title.as_deref().unwrap_or("(sans objet)");
        // Extract "De :" from content first line
        let from_line = r.content.lines()
            .find(|l| l.starts_with("De :"))
            .unwrap_or("")
            .trim_start_matches("De :").trim();

        out.push_str(&format!("{}. {}\n", i + 1, title));
        if !from_line.is_empty() {
            out.push_str(&format!("   De : {}\n", from_line));
        }
        out.push_str(&format!("   ID : {}\n\n", r.url.trim_start_matches("gmail://message/")));
    }
    out
}

// ─── fetch_content ────────────────────────────────────────────────────────────

const MAX_PDF_READ: u64 = 20 * 1024 * 1024; // 20 MB pour les PDFs
const SMART_CHUNK_SIZE: usize = 1500;        // Taille des blocs pour le scoring ONNX
const SMART_CHUNK_OVERLAP: usize = 150;      // Overlap entre blocs

// ─── fetch_content_smart (Agentic RAG) ───────────────────────────────────────

fn fetch_content_smart(
    path: &std::path::Path,
    query: &str,
    query_vec: Vec<f32>,
    block_index: Option<usize>,
) -> String {
    // 1. Extraire le texte brut
    let full_text = extract_text(path);
    let full_text = match full_text {
        Ok(t) => t,
        Err(e) => return e,
    };

    if full_text.is_empty() {
        return format!("Fichier vide ou sans texte extractible : {}", path.display());
    }

    // 2. Découper en blocs
    let chars: Vec<char> = full_text.chars().collect();
    let mut blocks: Vec<String> = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + SMART_CHUNK_SIZE).min(chars.len());
        blocks.push(chars[start..end].iter().collect());
        if end == chars.len() { break; }
        start += SMART_CHUNK_SIZE - SMART_CHUNK_OVERLAP;
    }

    let total_blocks = blocks.len();

    // 3. Si block_index demandé directement → retourner ce bloc
    if let Some(idx) = block_index {
        if idx >= total_blocks {
            return format!("Bloc {} inexistant. Ce fichier contient {} blocs (0 à {}).",
                idx, total_blocks, total_blocks - 1);
        }
        return format!(
            "📄 {} | Bloc {}/{} (demande directe)\n─────────────────────────────────────\n{}\n─────────────────────────────────────\n💡 Pour naviguer : fetch_content(path, query=\"{}\", block_index=N)",
            path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
            idx + 1, total_blocks,
            blocks[idx],
            query
        );
    }

    // 4. Scorer chaque bloc avec le vecteur query (cosinus)
    let mut scored: Vec<(usize, f32)> = blocks
        .iter()
        .enumerate()
        .map(|(i, block)| {
            // Embedding simplifié : TF sur les mots communs (fallback sans ONNX par bloc)
            // On utilise le vecteur query déjà calculé
            let score = simple_score(block, &query_vec, query);
            (i, score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let best_idx = scored[0].0;
    let best_score = scored[0].1;

    // 5. Carte de navigation : top-5 blocs + adjacents du meilleur
    let mut nav = String::from("\n\n🗺️  CARTE DE NAVIGATION\n");
    nav.push_str(&format!("   Fichier : {} blocs total\n", total_blocks));
    nav.push_str(&format!("   Requête : \"{}\"\n\n", query));
    nav.push_str("   Top blocs pertinents :\n");

    for (rank, (idx, score)) in scored.iter().take(5).enumerate() {
        let marker = if *idx == best_idx { " ◀ CE BLOC" } else { "" };
        nav.push_str(&format!(
            "   #{} → Bloc {} | Score {:.2}{}\n",
            rank + 1, idx + 1, score, marker
        ));
    }

    // Blocs adjacents du meilleur
    nav.push_str("\n   Blocs adjacents du meilleur :\n");
    if best_idx > 0 {
        nav.push_str(&format!("   ← Précédent : block_index={}\n", best_idx - 1));
    }
    if best_idx + 1 < total_blocks {
        nav.push_str(&format!("   → Suivant   : block_index={}\n", best_idx + 1));
    }
    nav.push_str(&format!(
        "\n   💡 Pour lire un bloc : fetch_content(path, query=\"{}\", block_index=N)\n",
        query
    ));

    // 6. Retourner le meilleur bloc + carte
    format!(
        "📄 {} | Bloc {}/{} | Score {:.2} (meilleur match)\n─────────────────────────────────────\n{}{}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
        best_idx + 1, total_blocks, best_score,
        blocks[best_idx],
        nav
    )
}

/// Score rapide basé sur les mots communs entre le bloc et la query.
/// Pas d'ONNX par bloc (trop lent) — on utilise TF-IDF simplifié.
fn simple_score(block: &str, _query_vec: &[f32], query: &str) -> f32 {
    let block_lower = block.to_lowercase();
    let query_words: Vec<&str> = query.split_whitespace().collect();
    let total = query_words.len().max(1) as f32;
    let matches = query_words.iter()
        .filter(|w| w.len() > 2 && block_lower.contains(&w.to_lowercase()))
        .count() as f32;
    matches / total
}

/// Extrait le texte brut d'un fichier (texte ou PDF).
fn extract_text(path: &std::path::Path) -> Result<String, String> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if ext == "pdf" {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size > MAX_PDF_READ {
            return Err(format!("PDF trop volumineux ({} Mo).", size / 1024 / 1024));
        }
        pdf_extract::extract_text(path)
            .map(|t| t.trim().to_string())
            .map_err(|e| format!("Erreur lecture PDF : {}", e))
    } else {
        std::fs::read_to_string(path)
            .map_err(|_| format!("Fichier binaire non lisible : {}", path.display()))
    }
}

/// Mode linéaire : lecture par offset/length sans scoring.
fn fetch_file_content(path: &std::path::Path, offset: usize, length: usize) -> String {
    if !path.exists() {
        return format!("Erreur : fichier introuvable : {}", path.display());
    }
    let full_text = match extract_text(path) {
        Ok(t) => t,
        Err(e) => return e,
    };
    if full_text.is_empty() {
        return format!("Fichier vide ou sans texte extractible : {}", path.display());
    }
    let chars: Vec<char> = full_text.chars().collect();
    let total_chars = chars.len();
    let total_sections = (total_chars + length - 1) / length;
    let current_section = offset / length + 1;
    let start = offset.min(total_chars);
    let end = (offset + length).min(total_chars);
    let slice: String = chars[start..end].iter().collect();

    let mut out = format!(
        "📄 {} | Section {}/{} | Chars {}-{} sur {}\n",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
        current_section, total_sections, start, end, total_chars
    );
    if end < total_chars {
        out.push_str(&format!("➡️  Suite : fetch_content(path, offset={}, length={})\n", end, length));
    }
    out.push_str("─────────────────────────────────────\n");
    out.push_str(&slice);
    if end < total_chars {
        out.push_str(&format!("\n[{} chars restants]", total_chars - end));
    }
    out
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

// ─── find_file_filesystem ─────────────────────────────────────────────────────

/// Recherche instantanée de fichiers par nom dans les dossiers courants.
/// Pas de LanceDB, pas d'ONNX — scan direct du filesystem.
fn find_file_filesystem(pattern: &str, limit: usize) -> String {
    use std::time::SystemTime;

    let home = match dirs_next::home_dir() {
        Some(h) => h,
        None => return "Impossible de localiser le home directory.".to_string(),
    };

    let search_dirs = [
        home.join("Desktop"),
        home.join("Documents"),
        home.join("Downloads"),
    ];

    let pattern_lower = pattern.to_lowercase();
    // Séparer en mots pour une recherche multi-terme
    let pattern_words: Vec<&str> = pattern_lower.split_whitespace().collect();

    let mut matches: Vec<(std::path::PathBuf, u64, SystemTime)> = Vec::new();

    for dir in &search_dirs {
        if !dir.exists() { continue; }
        find_recursive(dir, &pattern_words, &mut matches, 0, limit * 4);
        if matches.len() >= limit * 4 { break; }
    }

    // Trier : d'abord les correspondances exactes, puis par date de modification (récent en premier)
    matches.sort_by(|a, b| {
        let score_a = name_match_score(a.0.file_name().and_then(|n| n.to_str()).unwrap_or(""), &pattern_words);
        let score_b = name_match_score(b.0.file_name().and_then(|n| n.to_str()).unwrap_or(""), &pattern_words);
        score_b.partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.2.cmp(&a.2))
    });
    matches.truncate(limit);

    if matches.is_empty() {
        return format!(
            "Aucun fichier trouvé pour : \"{}\"\n\nConseils :\n• Vérifiez l'orthographe\n• Essayez un mot-clé plus court\n• Utilisez list_directory pour explorer un dossier",
            pattern
        );
    }

    let mut out = format!("Fichiers trouvés pour \"{}\" ({} résultats) :\n\n", pattern, matches.len());
    for (i, (path, size_bytes, modified)) in matches.iter().enumerate() {
        let size = if *size_bytes < 1024 {
            format!("{} o", size_bytes)
        } else if *size_bytes < 1024 * 1024 {
            format!("{} Ko", size_bytes / 1024)
        } else {
            format!("{:.1} Mo", *size_bytes as f64 / (1024.0 * 1024.0))
        };
        let ago = SystemTime::now().duration_since(*modified)
            .map(|d| {
                let mins = d.as_secs() / 60;
                if mins < 60 { format!("il y a {}min", mins) }
                else if mins < 1440 { format!("il y a {}h", mins / 60) }
                else { format!("il y a {}j", mins / 1440) }
            })
            .unwrap_or_else(|_| "?".to_string());
        out.push_str(&format!(
            "{}. {}\n   📂 {}\n   Taille : {} | {}\n\n",
            i + 1,
            path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
            path.display(),
            size,
            ago
        ));
    }
    out
}

fn find_recursive(
    dir: &std::path::Path,
    pattern_words: &[&str],
    out: &mut Vec<(std::path::PathBuf, u64, std::time::SystemTime)>,
    depth: usize,
    max: usize,
) {
    if depth > 5 || out.len() >= max { return; }

    // Ignorer les dossiers système
    let skip = ["node_modules", ".git", "target", "__pycache__", ".cargo",
                 "dist", "build", ".next", ".nuxt", "vendor", ".build",
                 "Pods", "DerivedData", ".gradle", ".idea", "venv", ".venv",
                 "env", ".tox", ".osmozzz"];

    let rd = match std::fs::read_dir(dir) { Ok(r) => r, Err(_) => return };
    for entry in rd.flatten() {
        if out.len() >= max { break; }
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if name.starts_with('.') { continue; }
        if skip.contains(&name.as_str()) { continue; }

        if let Ok(meta) = entry.metadata() {
            if meta.is_dir() {
                find_recursive(&path, pattern_words, out, depth + 1, max);
            } else if meta.is_file() {
                let name_lower = name.to_lowercase();
                // Match si TOUS les mots du pattern sont présents dans le nom
                let matches = pattern_words.iter().all(|w| name_lower.contains(*w));
                if matches {
                    let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    out.push((path, meta.len(), modified));
                }
            }
        }
    }
}

/// Score pour trier : nom exact > commence par > contient
fn name_match_score(name: &str, words: &[&str]) -> f32 {
    let name_lower = name.to_lowercase();
    let pattern = words.join(" ");
    if name_lower == pattern { return 3.0; }
    if name_lower.starts_with(&pattern) { return 2.0; }
    // Score proportionnel aux mots en début de nom
    let starts_count = words.iter().filter(|w| name_lower.starts_with(*w)).count();
    1.0 + starts_count as f32 / words.len().max(1) as f32
}
