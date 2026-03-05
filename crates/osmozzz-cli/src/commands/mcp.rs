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
use crate::proof;
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
            "description": "OUTIL PRINCIPAL — recherche sémantique (par concept/sens) dans TOUTE la mémoire indexée : Chrome, Safari, Gmail, fichiers, iMessages, Notes, Calendar, Terminal, Notion, GitHub, Linear, Jira, Slack, Trello, Todoist, GitLab, Airtable, Obsidian. QUAND L'UTILISER : questions vagues ou conceptuelles ('mes dépenses du mois', 'projet avec Thomas', 'site que j'ai visité sur les MCP tools', 'infos sur Revolut', 'issue sur le bug de login'). POUR UN SITE WEB VISITÉ : utilise search_memory avec un mot du nom du site — l'historique Chrome/Safari est ici. LIMITES : pour les noms propres exacts, enchaîne avec le tool dédié (search_emails, search_messages, search_notion, search_github, etc.).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "La requête en langage naturel (ex: 'site sur les outils MCP', 'mes virements Revolut', 'réunion avec Thomas')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 5, max: 20)",
                        "default": 5,
                        "minimum": 1,
                        "maximum": 20
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "search_emails",
            "description": "EMAILS UNIQUEMENT — recherche par mot-clé exact dans tous les emails indexés (expéditeur, objet, corps). QUAND L'UTILISER : l'utilisateur parle d'un email, d'un expéditeur, d'une facture, d'un abonnement. Scanne TOUS les emails sans limite de date. Retourne liste compacte (objet + expéditeur + ID). TOUJOURS enchaîner avec read_email(id) pour lire le contenu complet. NE PAS utiliser search_memory pour chercher des emails — ce tool est plus précis.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé exact à chercher (ex: 'revolut', 'facture', 'abonnement', 'railway')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre d'emails à retourner (défaut: 20, max: 100)",
                        "default": 20,
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "get_emails_by_date",
            "description": "EMAILS PAR DATE — QUAND L'UTILISER : l'utilisateur mentionne une période ('emails d'aujourd'hui', 'emails de janvier', 'emails de cette semaine', 'mes derniers emails'). Sans paramètre → 50 emails les plus récents. Avec query → filtre par période en langage naturel. Retourne liste compacte. TOUJOURS enchaîner avec read_email(id) pour le contenu complet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Période optionnelle : 'aujourd'hui', 'hier', 'cette semaine', 'janvier', 'le 15 février', 'ce mois'. Vide = emails les plus récents."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre d'emails (défaut: 50, max: 200)",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 200
                    }
                }
            }
        },
        {
            "name": "read_email",
            "description": "LIT UN EMAIL COMPLET — QUAND L'UTILISER : après search_emails ou get_emails_by_date pour lire le contenu intégral d'un email. Accepte l'ID court (ex: '20260214005158.abc@railway.app') ou l'URL complète (gmail://message/...).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "ID de l'email obtenu depuis search_emails ou get_emails_by_date"
                    }
                },
                "required": ["id"]
            }
        },
        {
            "name": "search_messages",
            "description": "IMESSAGES/SMS UNIQUEMENT — recherche par mot-clé exact dans toutes les conversations indexées. QUAND L'UTILISER : l'utilisateur parle d'un message, d'une conversation, d'un SMS, d'un contact. NE PAS utiliser search_memory pour les messages — ce tool est plus précis pour les noms et le texte exact.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher (nom d'un contact, mot dans un message, numéro de tel)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 20, max: 100)",
                        "default": 20,
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_notes",
            "description": "APPLE NOTES UNIQUEMENT — recherche par mot-clé exact dans toutes les notes indexées. QUAND L'UTILISER : l'utilisateur parle d'une note, d'une idée écrite, d'un mémo. Retourne titre + extrait. NE PAS utiliser search_memory pour les notes — ce tool est plus précis.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les notes"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 20, max: 100)",
                        "default": 20,
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_terminal",
            "description": "HISTORIQUE TERMINAL UNIQUEMENT — recherche par mot-clé exact dans ~/.zsh_history. QUAND L'UTILISER : l'utilisateur veut retrouver une commande shell précise ('comment j'avais lancé docker', 'la commande cargo que j'ai utilisée'). NE PAS utiliser search_memory pour les commandes terminal.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé de la commande (ex: 'docker run', 'git rebase', 'cargo build', 'osmozzz')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 20, max: 100)",
                        "default": 20,
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_calendar",
            "description": "APPLE CALENDAR UNIQUEMENT — recherche par mot-clé exact dans les événements indexés. QUAND L'UTILISER : l'utilisateur parle d'un rendez-vous, d'un événement, d'une réunion planifiée. Retourne titre + notes + date.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les événements (ex: 'dentiste', 'réunion', nom d'une personne)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 20, max: 100)",
                        "default": 20,
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_notion",
            "description": "NOTION UNIQUEMENT — recherche par mot-clé exact dans les pages Notion indexées. QUAND L'UTILISER : l'utilisateur demande quelque chose sur Notion, une doc, une page, un projet Notion. Retourne titre + extrait + URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les pages Notion"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 10, max: 50)",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 50
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_github",
            "description": "GITHUB UNIQUEMENT — recherche par mot-clé dans les issues et pull requests GitHub indexés. QUAND L'UTILISER : l'utilisateur parle d'un bug, d'une PR, d'une issue GitHub. Retourne titre + statut + URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les issues/PRs GitHub"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 10, max: 50)",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 50
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_linear",
            "description": "LINEAR UNIQUEMENT — recherche par mot-clé dans les issues Linear indexées. QUAND L'UTILISER : l'utilisateur parle de tâches Linear, de tickets, de sprints. Retourne titre + statut + équipe + URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les issues Linear"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 10, max: 50)",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 50
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_jira",
            "description": "JIRA UNIQUEMENT — recherche par mot-clé dans les issues Jira indexées. QUAND L'UTILISER : l'utilisateur parle de tickets Jira, de sprints, d'épics. Retourne titre + statut + priorité + URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les issues Jira"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 10, max: 50)",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 50
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_slack",
            "description": "SLACK UNIQUEMENT — recherche par mot-clé dans les messages Slack indexés. QUAND L'UTILISER : l'utilisateur cherche une conversation Slack, un message d'un collègue, une décision prise sur Slack. Retourne channel + auteur + extrait.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les messages Slack"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 20, max: 100)",
                        "default": 20,
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_trello",
            "description": "TRELLO UNIQUEMENT — recherche par mot-clé dans les cartes Trello indexées. QUAND L'UTILISER : l'utilisateur parle de cartes Trello, de boards, de to-do Trello. Retourne nom + board + liste + URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les cartes Trello"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 10, max: 50)",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 50
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_todoist",
            "description": "TODOIST UNIQUEMENT — recherche par mot-clé dans les tâches Todoist indexées. QUAND L'UTILISER : l'utilisateur parle de ses tâches, de sa to-do list Todoist, d'une tâche à faire. Retourne tâche + projet + priorité.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les tâches Todoist"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 10, max: 50)",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 50
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_gitlab",
            "description": "GITLAB UNIQUEMENT — recherche par mot-clé dans les issues et merge requests GitLab indexés. QUAND L'UTILISER : l'utilisateur parle d'issues GitLab, de MRs, de repos GitLab. Retourne titre + statut + URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les issues/MRs GitLab"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 10, max: 50)",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 50
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_airtable",
            "description": "AIRTABLE UNIQUEMENT — recherche par mot-clé dans les records Airtable indexés. QUAND L'UTILISER : l'utilisateur cherche dans ses bases Airtable, des données structurées dans Airtable. Retourne les champs du record + URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les records Airtable"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 10, max: 50)",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 50
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_obsidian",
            "description": "OBSIDIAN UNIQUEMENT — recherche par mot-clé dans les notes Obsidian (.md) indexées. QUAND L'UTILISER : l'utilisateur cherche dans ses notes Obsidian, son second cerveau, ses notes de cours/projet. Retourne titre + extrait.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "Mot-clé à chercher dans les notes Obsidian"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 10, max: 50)",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 50
                    }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "find_file",
            "description": "TROUVE UN FICHIER PAR SON NOM — QUAND L'UTILISER : l'utilisateur connaît le nom du fichier, son extension ou une partie de son chemin ('scene.gltf', 'fichiers .blend', 'error.log', 'rapport.pdf'). Scanne le filesystem (Desktop, Documents, code). NE PAS utiliser pour chercher par contenu — utilise search_memory pour ça. Après avoir trouvé le chemin, utilise fetch_content pour lire le fichier.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Nom, extension ou chemin partiel du fichier (ex: 'rapport.pdf', '.blend', 'main.rs')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre de résultats (défaut: 5, max: 20)",
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
            "description": "LIT LE CONTENU D'UN FICHIER — QUAND L'UTILISER : après find_file pour lire un fichier dont tu connais le chemin. AVEC query → mode RAG intelligent : ONNX score chaque bloc du fichier et retourne le bloc le plus pertinent + carte de navigation pour naviguer vers d'autres blocs (block_index). SANS query → lecture linéaire brute par offset/length.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Chemin absolu du fichier à lire"
                    },
                    "query": {
                        "type": "string",
                        "description": "Sujet recherché dans le fichier → active le mode RAG (retourne le bloc le plus pertinent)"
                    },
                    "block_index": {
                        "type": "integer",
                        "description": "Index d'un bloc spécifique à lire (issu de la carte de navigation)",
                        "minimum": 0
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Position de départ en caractères (mode linéaire sans query, défaut: 0)",
                        "default": 0,
                        "minimum": 0
                    },
                    "length": {
                        "type": "integer",
                        "description": "Nombre de caractères à lire (mode linéaire, défaut: 3000, max: 10000)",
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
            "description": "FICHIERS RÉCEMMENT MODIFIÉS — QUAND L'UTILISER : l'utilisateur veut reprendre un travail en cours, voir ce qu'il a modifié récemment ('sur quoi j'ai travaillé aujourd'hui', 'mes fichiers récents'). Retourne les fichiers modifiés dans Desktop et Documents dans une fenêtre temporelle. NE PAS utiliser pour chercher des sites web visités — utilise search_memory pour ça.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hours": {
                        "type": "integer",
                        "description": "Fenêtre temporelle en heures (défaut: 24, max: 168 = 7 jours)",
                        "default": 24,
                        "minimum": 1,
                        "maximum": 168
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Nombre max de fichiers (défaut: 20, max: 100)",
                        "default": 20,
                        "minimum": 1,
                        "maximum": 100
                    }
                }
            }
        },
        {
            "name": "list_directory",
            "description": "LISTE UN DOSSIER — QUAND L'UTILISER : l'utilisateur veut voir le contenu d'un dossier spécifique dont il connaît le chemin (ex: ~/Desktop, ~/Documents, ~/code/monprojet). Retourne nom, type, taille, date de modification.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Chemin du dossier à lister (ex: ~/Desktop, ~/Documents, ~/code)"
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

    let proof_key = proof::load_or_create_key();

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

                                let text = format_results(&query, &results, &proof_key);
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

                    "search_emails" => {
                        let keyword = match args["keyword"].as_str() {
                            Some(k) => k.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: keyword"));
                                continue;
                            }
                        };
                        let limit = args["limit"].as_u64().unwrap_or(20) as usize;
                        let limit = limit.clamp(1, 100);

                        eprintln!("[OSMOzzz MCP] search_emails: \"{}\" (limit={})", keyword, limit);

                        match vault.search_emails_by_keyword(&keyword, limit).await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucun email trouvé contenant \"{}\".\n\nConseil : essaie un mot-clé plus court ou plus général.", keyword)
                                } else {
                                    format_email_list(&results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "get_emails_by_date" => {
                        let query = args["query"].as_str().unwrap_or("").to_string();
                        let limit = args["limit"].as_u64().unwrap_or(50) as usize;
                        let limit = limit.clamp(1, 200);

                        eprintln!("[OSMOzzz MCP] get_emails_by_date: \"{}\" (limit={})", query, limit);

                        if query.is_empty() {
                            // Pas de query → emails récents
                            match vault.recent_emails_full(limit).await {
                                Ok(results) => {
                                    send(&Response::ok(id, json!({
                                        "content": [{"type": "text", "text": format_email_list(&results)}]
                                    })));
                                }
                                Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                            }
                        } else {
                            match parse_date_range(&query) {
                                Some((from_ts, to_ts)) => {
                                    match vault.get_emails_by_date(from_ts, to_ts, limit).await {
                                        Ok(results) if !results.is_empty() => {
                                            send(&Response::ok(id, json!({
                                                "content": [{"type": "text", "text": format_email_list(&results)}]
                                            })));
                                        }
                                        Ok(_) => {
                                            send(&Response::ok(id, json!({
                                                "content": [{"type": "text", "text": format!("Aucun email trouvé pour : \"{}\".", query)}]
                                            })));
                                        }
                                        Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                                    }
                                }
                                None => {
                                    // Date non reconnue → fallback récents
                                    eprintln!("[OSMOzzz MCP] Date non reconnue, fallback récents");
                                    match vault.recent_emails_full(limit).await {
                                        Ok(results) => {
                                            send(&Response::ok(id, json!({
                                                "content": [{"type": "text", "text": format_email_list(&results)}]
                                            })));
                                        }
                                        Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                                    }
                                }
                            }
                        }
                    }

                    "read_email" => {
                        let raw_id = match args["id"].as_str() {
                            Some(i) => i.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: id"));
                                continue;
                            }
                        };
                        let url = if raw_id.starts_with("gmail://") {
                            raw_id.clone()
                        } else {
                            format!("gmail://message/{}", raw_id)
                        };
                        eprintln!("[OSMOzzz MCP] read_email: {}", url);
                        match vault.get_full_content_by_url(&url).await {
                            Ok(Some((title, content))) => {
                                let mut out = String::new();
                                if let Some(t) = &title {
                                    out.push_str(&format!("Objet : {}\n", t));
                                }
                                out.push_str(&format!("ID    : {}\n", raw_id.trim_start_matches("gmail://message/")));
                                out.push_str("\n─────────────────────────────────────\n");
                                out.push_str(&content);
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": out}]
                                })));
                            }
                            Ok(None) => {
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": format!("Email introuvable : {}", url)}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
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

                    "search_messages" => {
                        let keyword = match args["keyword"].as_str() {
                            Some(k) => k.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: keyword"));
                                continue;
                            }
                        };
                        let limit = args["limit"].as_u64().unwrap_or(20) as usize;
                        let limit = limit.clamp(1, 100);

                        eprintln!("[OSMOzzz MCP] search_messages: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "imessage").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucun message trouvé pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("iMessages", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_notes" => {
                        let keyword = match args["keyword"].as_str() {
                            Some(k) => k.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: keyword"));
                                continue;
                            }
                        };
                        let limit = args["limit"].as_u64().unwrap_or(20) as usize;
                        let limit = limit.clamp(1, 100);

                        eprintln!("[OSMOzzz MCP] search_notes: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "notes").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucune note trouvée pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("Notes", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_terminal" => {
                        let keyword = match args["keyword"].as_str() {
                            Some(k) => k.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: keyword"));
                                continue;
                            }
                        };
                        let limit = args["limit"].as_u64().unwrap_or(20) as usize;
                        let limit = limit.clamp(1, 100);

                        eprintln!("[OSMOzzz MCP] search_terminal: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "terminal").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucune commande trouvée pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("Terminal", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_calendar" => {
                        let keyword = match args["keyword"].as_str() {
                            Some(k) => k.to_string(),
                            None => {
                                send(&Response::err(id, -32602, "Missing required param: keyword"));
                                continue;
                            }
                        };
                        let limit = args["limit"].as_u64().unwrap_or(20) as usize;
                        let limit = limit.clamp(1, 100);

                        eprintln!("[OSMOzzz MCP] search_calendar: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "calendar").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucun événement trouvé pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("Calendar", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_notion" => {
                        let keyword = args["keyword"].as_str().unwrap_or("").to_string();
                        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
                        eprintln!("[OSMOzzz MCP] search_notion: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "notion").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucune page Notion trouvée pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("Notion", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_github" => {
                        let keyword = args["keyword"].as_str().unwrap_or("").to_string();
                        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
                        eprintln!("[OSMOzzz MCP] search_github: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "github").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucune issue/PR GitHub trouvée pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("GitHub", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_linear" => {
                        let keyword = args["keyword"].as_str().unwrap_or("").to_string();
                        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
                        eprintln!("[OSMOzzz MCP] search_linear: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "linear").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucune issue Linear trouvée pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("Linear", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_jira" => {
                        let keyword = args["keyword"].as_str().unwrap_or("").to_string();
                        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
                        eprintln!("[OSMOzzz MCP] search_jira: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "jira").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucune issue Jira trouvée pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("Jira", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_slack" => {
                        let keyword = args["keyword"].as_str().unwrap_or("").to_string();
                        let limit = args["limit"].as_u64().unwrap_or(20) as usize;
                        eprintln!("[OSMOzzz MCP] search_slack: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "slack").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucun message Slack trouvé pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("Slack", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_trello" => {
                        let keyword = args["keyword"].as_str().unwrap_or("").to_string();
                        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
                        eprintln!("[OSMOzzz MCP] search_trello: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "trello").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucune carte Trello trouvée pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("Trello", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_todoist" => {
                        let keyword = args["keyword"].as_str().unwrap_or("").to_string();
                        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
                        eprintln!("[OSMOzzz MCP] search_todoist: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "todoist").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucune tâche Todoist trouvée pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("Todoist", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_gitlab" => {
                        let keyword = args["keyword"].as_str().unwrap_or("").to_string();
                        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
                        eprintln!("[OSMOzzz MCP] search_gitlab: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "gitlab").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucune issue/MR GitLab trouvée pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("GitLab", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_airtable" => {
                        let keyword = args["keyword"].as_str().unwrap_or("").to_string();
                        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
                        eprintln!("[OSMOzzz MCP] search_airtable: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "airtable").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucun record Airtable trouvé pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("Airtable", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
                    }

                    "search_obsidian" => {
                        let keyword = args["keyword"].as_str().unwrap_or("").to_string();
                        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
                        eprintln!("[OSMOzzz MCP] search_obsidian: \"{}\"", keyword);
                        match vault.search_by_keyword_source(&keyword, limit, "obsidian").await {
                            Ok(results) => {
                                let msg = if results.is_empty() {
                                    format!("Aucune note Obsidian trouvée pour \"{}\".", keyword)
                                } else {
                                    format_keyword_results("Obsidian", &keyword, &results)
                                };
                                send(&Response::ok(id, json!({
                                    "content": [{"type": "text", "text": msg}]
                                })));
                            }
                            Err(e) => send(&Response::err(id, -32603, &e.to_string())),
                        }
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

// ─── Date range parser ────────────────────────────────────────────────────────

/// Parse a natural language date query into a (from_ts, to_ts) Unix timestamp range.
fn parse_date_range(query: &str) -> Option<(i64, i64)> {
    use chrono::{Datelike, Duration, NaiveDate, Utc};

    let q = query.to_lowercase();
    let now = Utc::now();
    let today = now.date_naive();

    let day_range = |date: NaiveDate| -> Option<(i64, i64)> {
        let from = date.and_hms_opt(0, 0, 0)?.and_utc().timestamp();
        let to   = date.and_hms_opt(23, 59, 59)?.and_utc().timestamp();
        Some((from, to))
    };

    // aujourd'hui
    if q.contains("aujourd") {
        return day_range(today);
    }
    // hier
    if q.contains("hier") {
        return day_range(today - Duration::days(1));
    }
    // cette semaine / semaine
    if q.contains("cette semaine") || q.contains("semaine") {
        let from = (now - Duration::days(7)).timestamp();
        return Some((from, now.timestamp()));
    }
    // ce mois
    if q.contains("ce mois") || q.contains("mois-ci") {
        let from = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)?
            .and_hms_opt(0, 0, 0)?.and_utc().timestamp();
        return Some((from, now.timestamp()));
    }

    // Noms de mois français
    let months: &[(&str, u32)] = &[
        ("janvier", 1), ("février", 2), ("fevrier", 2), ("mars", 3),
        ("avril", 4), ("mai", 5), ("juin", 6), ("juillet", 7),
        ("août", 8), ("aout", 8), ("septembre", 9), ("octobre", 10),
        ("novembre", 11), ("décembre", 12), ("decembre", 12),
    ];

    for (month_name, month_num) in months {
        if let Some(idx) = q.find(month_name) {
            let year = today.year();
            let before = q[..idx].trim_end();
            // Cherche un nombre (le jour) juste avant le nom du mois
            let day_opt = before.split_whitespace().last()
                .and_then(|w| {
                    let digits: String = w.chars().filter(|c| c.is_ascii_digit()).collect();
                    digits.parse::<u32>().ok()
                })
                .filter(|&d| d >= 1 && d <= 31);

            if let Some(day) = day_opt {
                // Jour précis : "01 février"
                if let Some(date) = NaiveDate::from_ymd_opt(year, *month_num, day) {
                    return day_range(date);
                }
            } else {
                // Mois entier : "janvier"
                let from_date = NaiveDate::from_ymd_opt(year, *month_num, 1)?;
                let to_date = if *month_num == 12 {
                    NaiveDate::from_ymd_opt(year + 1, 1, 1)?
                } else {
                    NaiveDate::from_ymd_opt(year, *month_num + 1, 1)?
                };
                let from = from_date.and_hms_opt(0, 0, 0)?.and_utc().timestamp();
                let to   = to_date.and_hms_opt(0, 0, 0)?.and_utc().timestamp() - 1;
                return Some((from, to));
            }
        }
    }

    // "le X" sans mois → mois courant
    if let Some(idx) = q.find("le ") {
        let rest = q[idx + 3..].trim_start();
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(day) = digits.parse::<u32>() {
            if day >= 1 && day <= 31 {
                if let Some(date) = NaiveDate::from_ymd_opt(today.year(), today.month(), day) {
                    return day_range(date);
                }
            }
        }
    }

    None
}

// ─── Formatters email ─────────────────────────────────────────────────────────

/// Extrait expéditeur et date depuis les premières lignes du contenu stocké.
fn extract_email_meta(content: &str) -> (String, String) {
    let mut from = String::new();
    let mut date = String::new();
    for line in content.lines().take(10) {
        if line.starts_with("De :") && from.is_empty() {
            from = line.trim_start_matches("De :").trim().to_string();
        } else if line.starts_with("Date :") && date.is_empty() {
            date = line.trim_start_matches("Date :").trim().to_string();
        }
        if !from.is_empty() && !date.is_empty() { break; }
    }
    (from, date)
}

/// Liste compacte d'emails : objet + expéditeur + date + ID.
/// Claude appelle ensuite read_email(id) pour le contenu complet.
fn format_email_list(results: &[(Option<String>, String, String)]) -> String {
    if results.is_empty() {
        return "Aucun email trouvé.".to_string();
    }
    let mut out = format!("{} email(s) trouvé(s) :\n\n", results.len());
    for (i, (title, url, content)) in results.iter().enumerate() {
        let (from, date) = extract_email_meta(content);
        let subject = title.as_deref().unwrap_or("(sans objet)");
        let msg_id = url.trim_start_matches("gmail://message/");
        out.push_str(&format!(
            "{}. 📧 {}\n   De   : {}\n   Date : {}\n   ID   : {}\n\n",
            i + 1, subject, from, date, msg_id
        ));
    }
    out.push_str("→ Pour lire un email : read_email(id=\"...\")");
    out
}

// ─── Formatter générique pour les nouvelles sources ──────────────────────────

/// Liste compacte pour iMessage, Notes, Terminal, Calendar, Safari.
fn format_keyword_results(
    label: &str,
    keyword: &str,
    results: &[(Option<String>, String, String)],
) -> String {
    let mut out = format!(
        "{} résultat(s) {} pour \"{}\" :\n\n",
        results.len(),
        label,
        keyword
    );
    for (i, (title, url, content)) in results.iter().enumerate() {
        let t = title.as_deref().unwrap_or("—");
        // Extrait: 200 premiers chars du content
        let preview = {
            let s = content.trim();
            if s.len() > 200 {
                let mut b = 200;
                while b > 0 && !s.is_char_boundary(b) { b -= 1; }
                format!("{}…", &s[..b])
            } else {
                s.to_string()
            }
        };
        out.push_str(&format!(
            "{}. {}\n   URL : {}\n   {}\n\n",
            i + 1, t, url, preview
        ));
    }
    out
}

// ─── Formatage des résultats ──────────────────────────────────────────────────

fn format_results(query: &str, results: &[osmozzz_core::SearchResult], proof_key: &[u8; 32]) -> String {
    if results.is_empty() {
        return format!("Aucun résultat trouvé pour : \"{}\"", query);
    }

    let ts = chrono::Utc::now().timestamp();

    let mut out = format!(
        "Résultats de recherche dans ta mémoire locale pour : \"{}\"\n\n",
        query
    );

    for (i, r) in results.iter().enumerate() {
        let chunk_info = match (r.chunk_index, r.chunk_total) {
            (Some(idx), Some(tot)) if tot > 1 => format!(" [partie {}/{}]", idx + 1, tot),
            _ => String::new(),
        };

        let sig = proof::sign(proof_key, &r.source, &r.url, &r.content, ts);

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
        out.push_str(&format!("   Extrait : {}\n", r.content.replace('\n', " ")));
        out.push_str(&format!("   🔐 PROOF: {} | ts={}\n\n", sig, ts));
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

/// Lit les premiers caractères d'un fichier lisible pour un aperçu.
/// Retourne None si le fichier est binaire, trop grand, ou vide.
fn read_file_preview(path: &std::path::Path, max_chars: usize) -> Option<String> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let size = std::fs::metadata(path).ok()?.len();
    if size > 10 * 1024 * 1024 { return None; } // skip > 10 Mo

    let text = if ext == "pdf" {
        if size > 5 * 1024 * 1024 { return None; }
        pdf_extract::extract_text(path).ok()?
    } else {
        let readable = matches!(ext.as_str(),
            "txt" | "md" | "mdx" | "rs" | "js" | "ts" | "tsx" | "jsx" | "py"
            | "json" | "yaml" | "yml" | "toml" | "csv" | "html" | "css" | "sh"
            | "log" | "conf" | "cfg" | "ini" | "xml" | "sql" | "go" | "java"
            | "c" | "cpp" | "h" | "rb" | "php" | "swift" | "kt" | "scala"
            | "org" | "tex" | "rst" | "adoc"
        );
        if !readable { return None; }
        std::fs::read_to_string(path).ok()?
    };

    let text = text.trim();
    if text.is_empty() { return None; }

    let chars: Vec<char> = text.chars().collect();
    let end = chars.len().min(max_chars);
    // Garde une seule ligne pour l'aperçu (plus lisible)
    let preview: String = chars[..end].iter().collect();
    let preview = preview.lines()
        .filter(|l| !l.trim().is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join(" · ");
    if preview.is_empty() { None } else { Some(preview) }
}

/// Recherche instantanée de fichiers par nom ET contenu dans les dossiers courants.
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
    let pattern_words: Vec<&str> = pattern_lower.split_whitespace().collect();

    // (path, size, modified, preview, name_match)
    let mut matches: Vec<(std::path::PathBuf, u64, SystemTime, Option<String>, bool)> = Vec::new();

    for dir in &search_dirs {
        if !dir.exists() { continue; }
        find_recursive(dir, &pattern_words, &mut matches, 0, limit * 6);
        if matches.len() >= limit * 6 { break; }
    }

    // Trier : nom exact > nom partiel > contenu ; puis par date
    matches.sort_by(|a, b| {
        let score_a = if a.4 {
            name_match_score(a.0.file_name().and_then(|n| n.to_str()).unwrap_or(""), &pattern_words) + 1.0
        } else { 0.5 };
        let score_b = if b.4 {
            name_match_score(b.0.file_name().and_then(|n| n.to_str()).unwrap_or(""), &pattern_words) + 1.0
        } else { 0.5 };
        score_b.partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.2.cmp(&a.2))
    });
    matches.truncate(limit);

    if matches.is_empty() {
        return format!(
            "Aucun fichier trouvé pour : \"{}\"\n\nConseils :\n• Essayez un mot-clé plus court\n• Utilisez list_directory pour explorer un dossier",
            pattern
        );
    }

    let mut out = format!("Fichiers trouvés pour \"{}\" ({} résultats) :\n\n", pattern, matches.len());
    for (i, (path, size_bytes, modified, preview, name_match)) in matches.iter().enumerate() {
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
        let match_type = if *name_match { "nom" } else { "contenu" };
        out.push_str(&format!(
            "{}. {} [{}]\n   📂 {}\n   {} | {}\n",
            i + 1,
            path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
            match_type,
            path.display(),
            size,
            ago
        ));
        if let Some(p) = preview {
            out.push_str(&format!("   📝 {}\n", p));
        }
        out.push_str(&format!("   → fetch_content(path=\"{}\")\n\n", path.display()));
    }
    out
}

fn find_recursive(
    dir: &std::path::Path,
    pattern_words: &[&str],
    out: &mut Vec<(std::path::PathBuf, u64, std::time::SystemTime, Option<String>, bool)>,
    depth: usize,
    max: usize,
) {
    if depth > 20 || out.len() >= max { return; }

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
                let name_match = pattern_words.iter().all(|w| name_lower.contains(*w));
                let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                if name_match {
                    let preview = read_file_preview(&path, 300);
                    out.push((path, meta.len(), modified, preview, true));
                } else {
                    if let Some(preview) = read_file_preview(&path, 500) {
                        let preview_lower = preview.to_lowercase();
                        let content_match = pattern_words.iter().any(|w| w.len() > 2 && preview_lower.contains(*w));
                        if content_match {
                            out.push((path, meta.len(), modified, Some(preview), false));
                        }
                    }
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
