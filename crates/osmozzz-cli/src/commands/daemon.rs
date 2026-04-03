/// Service watcher autonome — indépendant du MCP et de l'IA.
///
/// Lance la surveillance de ~/Desktop et ~/Documents en arrière-plan.
/// Indexe silencieusement chaque fichier créé ou modifié dans LanceDB.
/// Le serveur MCP (et toute autre app future) lit la même base sans conflit.
///
/// Lancement : osmozzz daemon
/// Arrêt propre : Ctrl+C
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use osmozzz_core::{Embedder, Harvester, OsmozzError};
use osmozzz_embedder::Vault;
use osmozzz_harvester::{
    start_watcher, ArcHarvester, ContactsHarvester,
    TerminalHarvester, WatchEvent,
};
#[cfg(target_os = "macos")]
use osmozzz_harvester::{CalendarHarvester, IMessageHarvester, NotesHarvester, SafariHarvester};

use osmozzz_api;
use osmozzz_embedder::Blacklist;
use osmozzz_p2p::{P2pNode, ToolCallResult, node::{P2pEvent, DEFAULT_P2P_PORT}};

use crate::config::Config;

const DASHBOARD_PORT: u16 = 7878;

// Auto-syncs locaux désactivés — réactiver en décommentant les blocs tokio::spawn ci-dessous
const IMESSAGE_SYNC_INTERVAL_SECS: u64 = 60;
const SAFARI_SYNC_INTERVAL_SECS: u64 = 60;
const NOTES_SYNC_INTERVAL_SECS: u64 = 60;
const TERMINAL_SYNC_INTERVAL_SECS: u64 = 60;
const CALENDAR_SYNC_INTERVAL_SECS: u64 = 60;
const CONTACTS_SYNC_INTERVAL_SECS: u64 = 10 * 60;
const ARC_SYNC_INTERVAL_SECS: u64 = 60;


/// Copie les modèles ONNX dans ~/.osmozzz/models/ si absents.
/// Cherche dans les ancêtres du binaire (workspace dev) ou à côté du binaire.
fn ensure_models(cfg: &Config) {
    let dest_dir = cfg.data_dir.join("models");
    let model_dest = dest_dir.join("all-MiniLM-L6-v2.onnx");
    if model_dest.exists() {
        return; // déjà en place
    }

    // Cherche un dossier models/ contenant le fichier ONNX
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let source_dir = exe_dir
        .ancestors()
        .find(|p| p.join("models").join("all-MiniLM-L6-v2.onnx").exists())
        .map(|p| p.join("models"));

    let Some(src) = source_dir else {
        return; // pas trouvé — l'erreur sera levée par Vault::open
    };

    if let Err(e) = std::fs::create_dir_all(&dest_dir) {
        eprintln!("[OSMOzzz] Impossible de créer ~/.osmozzz/models/: {}", e);
        return;
    }

    for file in ["all-MiniLM-L6-v2.onnx", "tokenizer.json"] {
        let from = src.join(file);
        let to = dest_dir.join(file);
        if from.exists() {
            if let Err(e) = std::fs::copy(&from, &to) {
                eprintln!("[OSMOzzz] Copie {} échouée: {}", file, e);
            }
        }
    }
    eprintln!("[OSMOzzz] Modèles installés dans ~/.osmozzz/models/");
}

pub async fn run(cfg: Config) -> Result<()> {
    std::fs::create_dir_all(&cfg.data_dir)
        .context("Impossible de créer ~/.osmozzz")?;

    ensure_models(&cfg);

    eprintln!("[OSMOzzz Daemon] Initialisation du vault...");

    let vault = Arc::new(
        Vault::open(
            &cfg.model_path,
            &cfg.tokenizer_path,
            cfg.db_path.to_str().unwrap_or(".osmozzz/vault"),
        )
        .await
        .context("Impossible d'ouvrir le vault")?,
    );

    eprintln!("[OSMOzzz Daemon] Vault prêt.");

    // Créer la queue d'actions locale (Claude → dashboard, isolée du P2P)
    let action_queue = Arc::new(osmozzz_api::ActionQueue::new());
    // Queue dédiée aux approbations P2P (demandes de peers — page Réseau)
    let p2p_action_queue = Arc::new(osmozzz_api::ActionQueue::new());

    // Initialiser le nœud P2P
    let (p2p_event_tx, mut p2p_event_rx) = tokio::sync::mpsc::channel::<P2pEvent>(64);
    let display_name = std::env::var("USER").unwrap_or_else(|_| "OSMOzzz".to_string());
    let p2p_node = match P2pNode::new(&display_name, DEFAULT_P2P_PORT, p2p_event_tx).await {
        Ok(node) => {
            let node_clone = node.clone();
            tokio::spawn(async move {
                if let Err(e) = node_clone.start_server().await {
                    eprintln!("[P2P] Erreur serveur : {}", e);
                }
            });
            // Écoute les événements P2P (connexions, requêtes, approbations)
            let my_peer_id = node.identity.id.clone();
            let my_display_name = display_name.clone();
            let aq = Arc::clone(&p2p_action_queue);
            tokio::spawn(async move {
                while let Some(event) = p2p_event_rx.recv().await {
                    match event {
                        P2pEvent::PeerConnected { display_name, .. } => {
                            eprintln!("[P2P] {} connecté", display_name);
                        }
                        P2pEvent::PeerDisconnected { peer_id } => {
                            eprintln!("[P2P] Peer {} déconnecté", &peer_id[..8.min(peer_id.len())]);
                        }
                        P2pEvent::QueryReceived { peer_name, query, .. } => {
                            eprintln!("[P2P] {} a cherché : \"{}\"", peer_name, query);
                        }
                        P2pEvent::ToolCallAuto { peer_id: _, peer_name, tool_call, result_tx } => {
                            eprintln!("[P2P] Auto-exécution : {} pour {}", tool_call.tool_name, peer_name);
                            let my_id = my_peer_id.clone();
                            let my_name = my_display_name.clone();
                            tokio::spawn(async move {
                                let (res_text, err_text) = match crate::connectors::handle(&tool_call.tool_name, &tool_call.params).await {
                                    Some(Ok(text)) => (Some(text), None),
                                    Some(Err(e)) => (None, Some(e)),
                                    None => (None, Some(format!("Tool '{}' non trouvé", tool_call.tool_name))),
                                };
                                let _ = result_tx.send(ToolCallResult {
                                    request_id: tool_call.request_id,
                                    peer_id: my_id,
                                    peer_name: my_name,
                                    tool_name: tool_call.tool_name,
                                    result: res_text,
                                    error: err_text,
                                });
                            });
                        }
                        P2pEvent::ToolCallPending { peer_id: _, peer_name, tool_call, result_tx } => {
                            // Crée une action dans la file d'approbation du dashboard
                            use osmozzz_core::{ActionRequest, ActionStatus};
                            let action_id = uuid::Uuid::new_v4().to_string();
                            let preview = format!(
                                "[P2P — {}] {} — paramètres : {}",
                                peer_name,
                                tool_call.tool_name,
                                serde_json::to_string(&tool_call.params).unwrap_or_default()
                            );
                            let now = chrono::Utc::now().timestamp();
                            let action = ActionRequest {
                                id: action_id.clone(),
                                tool: format!("p2p:{}", tool_call.tool_name),
                                params: tool_call.params,
                                preview,
                                status: ActionStatus::Pending,
                                created_at: now,
                                expires_at: now + 300,
                                execution_result: None,
                                mcp_proxy: None,
                            };
                            eprintln!("[P2P] Approbation requise : {} par {}", tool_call.tool_name, peer_name);
                            aq.push(action);

                            // Attend la décision depuis la file d'approbation
                            let mut sub = aq.subscribe();
                            tokio::spawn(async move {
                                loop {
                                    match sub.recv().await {
                                        Ok(event) if event.action.id == action_id => {
                                            use osmozzz_core::ActionStatus::*;
                                            match event.action.status {
                                                Approved => {
                                                    let result = event.action.execution_result
                                                        .unwrap_or_else(|| "Approuvé".to_string());
                                                    let _ = result_tx.send(Some(result));
                                                    return;
                                                }
                                                Rejected | Expired => {
                                                    let _ = result_tx.send(None);
                                                    return;
                                                }
                                                _ => {}
                                            }
                                        }
                                        Err(_) => {
                                            let _ = result_tx.send(None);
                                            return;
                                        }
                                        _ => {}
                                    }
                                }
                            });
                        }
                    }
                }
            });
            eprintln!("[OSMOzzz Daemon] P2P démarré (port {})", DEFAULT_P2P_PORT);
            Some(node)
        }
        Err(e) => {
            eprintln!("[OSMOzzz Daemon] P2P désactivé : {}", e);
            None
        }
    };

    // Démarrer le serveur HTTP du dashboard
    let dashboard_vault = Arc::clone(&vault);
    let dashboard_p2p = p2p_node.clone();
    let dashboard_queue = Arc::clone(&action_queue);
    let dashboard_p2p_queue = Arc::clone(&p2p_action_queue);
    tokio::spawn(async move {
        if let Err(e) = osmozzz_api::start_server(dashboard_vault, dashboard_p2p, dashboard_queue, dashboard_p2p_queue, DASHBOARD_PORT).await {
            eprintln!("[OSMOzzz Daemon] Dashboard erreur: {}", e);
        }
    });

    // Health check périodique — guérit le vault si la connexion LanceDB est devenue
    // obsolète après une veille machine (sleep/wake macOS).
    let health_vault = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        interval.tick().await; // skip first tick (startup)
        loop {
            interval.tick().await;
            if let Err(e) = health_vault.health_check().await {
                eprintln!("[OSMOzzz Daemon] Vault stale ({}), reconnexion en cours…", e);
                match health_vault.heal().await {
                    Ok(_)  => eprintln!("[OSMOzzz Daemon] Vault reconnecté avec succès."),
                    Err(e) => eprintln!("[OSMOzzz Daemon] Échec reconnexion vault : {}", e),
                }
            }
        }
    });

    // Ouvrir le dashboard dans le navigateur par défaut après 500ms
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let url = format!("http://localhost:{}", DASHBOARD_PORT);
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&url).spawn();
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd").args(["/c", "start", &url]).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
        eprintln!("[OSMOzzz Daemon] Dashboard ouvert sur {}", url);
    });

    let watch_paths = default_watch_paths();
    if watch_paths.is_empty() {
        eprintln!("[OSMOzzz Daemon] Aucun dossier à surveiller (Desktop/Documents introuvables).");
        return Ok(());
    }

    eprintln!(
        "[OSMOzzz Daemon] Surveillance de {} dossier(s) :",
        watch_paths.len()
    );
    for p in &watch_paths {
        eprintln!("[OSMOzzz Daemon]   → {}", p.display());
    }
    // ── Auto-syncs locaux DÉSACTIVÉS — décommenter pour réactiver ──────────

    // iMessage auto-sync (macOS uniquement)
    // #[cfg(target_os = "macos")]
    // {
    //     let v = Arc::clone(&vault);
    //     tokio::spawn(async move {
    //         let mut interval = tokio::time::interval(
    //             tokio::time::Duration::from_secs(IMESSAGE_SYNC_INTERVAL_SECS)
    //         );
    //         loop {
    //             interval.tick().await;
    //             let docs = IMessageHarvester::new().harvest().await.unwrap_or_default();
    //             sync_docs(&v, "iMessage", docs).await;
    //         }
    //     });
    // }

    // Safari auto-sync (macOS uniquement)
    // #[cfg(target_os = "macos")]
    // {
    //     let v = Arc::clone(&vault);
    //     tokio::spawn(async move {
    //         let mut interval = tokio::time::interval(
    //             tokio::time::Duration::from_secs(SAFARI_SYNC_INTERVAL_SECS)
    //         );
    //         loop {
    //             interval.tick().await;
    //             let docs = SafariHarvester::new().harvest().await.unwrap_or_default();
    //             sync_docs(&v, "Safari", docs).await;
    //         }
    //     });
    // }

    // Notes auto-sync (macOS uniquement)
    // #[cfg(target_os = "macos")]
    // {
    //     let v = Arc::clone(&vault);
    //     tokio::spawn(async move {
    //         let mut interval = tokio::time::interval(
    //             tokio::time::Duration::from_secs(NOTES_SYNC_INTERVAL_SECS)
    //         );
    //         loop {
    //             interval.tick().await;
    //             let docs = NotesHarvester::new().harvest().await.unwrap_or_default();
    //             sync_docs(&v, "Notes", docs).await;
    //         }
    //     });
    // }

    // Terminal auto-sync
    // {
    //     let v = Arc::clone(&vault);
    //     tokio::spawn(async move {
    //         let mut interval = tokio::time::interval(
    //             tokio::time::Duration::from_secs(TERMINAL_SYNC_INTERVAL_SECS)
    //         );
    //         loop {
    //             interval.tick().await;
    //             let docs = TerminalHarvester::new().harvest().await.unwrap_or_default();
    //             sync_docs(&v, "Terminal", docs).await;
    //         }
    //     });
    // }

    // Calendar auto-sync (macOS uniquement)
    // #[cfg(target_os = "macos")]
    // {
    //     let v = Arc::clone(&vault);
    //     tokio::spawn(async move {
    //         let mut interval = tokio::time::interval(
    //             tokio::time::Duration::from_secs(CALENDAR_SYNC_INTERVAL_SECS)
    //         );
    //         loop {
    //             interval.tick().await;
    //             let docs = CalendarHarvester::new().harvest().await.unwrap_or_default();
    //             sync_docs(&v, "Calendar", docs).await;
    //         }
    //     });
    // }

    // Contacts auto-sync (10min)
    // {
    //     let v = Arc::clone(&vault);
    //     tokio::spawn(async move {
    //         let mut interval = tokio::time::interval(
    //             tokio::time::Duration::from_secs(CONTACTS_SYNC_INTERVAL_SECS)
    //         );
    //         loop {
    //             interval.tick().await;
    //             let docs = ContactsHarvester::new().harvest().await.unwrap_or_default();
    //             sync_docs(&v, "Contacts", docs).await;
    //         }
    //     });
    // }

    // Arc auto-sync (1min)
    // {
    //     let v = Arc::clone(&vault);
    //     tokio::spawn(async move {
    //         let mut interval = tokio::time::interval(
    //             tokio::time::Duration::from_secs(ARC_SYNC_INTERVAL_SECS)
    //         );
    //         loop {
    //             interval.tick().await;
    //             let docs = ArcHarvester::new().harvest().await.unwrap_or_default();
    //             sync_docs(&v, "Arc", docs).await;
    //         }
    //     });
    // }

    eprintln!("[OSMOzzz Daemon] En écoute... (Ctrl+C pour arrêter)");

    // ── FSEvents watcher DÉSACTIVÉ — décommenter pour réactiver ────────────
    // let mut rx = start_watcher(watch_paths);
    // loop {
    //     tokio::select! {
    //         event = rx.recv() => {
    //             match event {
    //                 Some(WatchEvent::Upsert(docs)) => {
    //                     for doc in &docs {
    //                         match vault.upsert(doc).await {
    //                             Ok(()) => {
    //                                 eprintln!(
    //                                     "[OSMOzzz Daemon] Indexé : {}",
    //                                     doc.url
    //                                 );
    //                             }
    //                             Err(OsmozzError::Storage(e)) if e.contains("duplicate") => {
    //                                 // Déjà présent en base, pas grave
    //                             }
    //                             Err(e) => {
    //                                 eprintln!(
    //                                     "[OSMOzzz Daemon] Erreur upsert {}: {}",
    //                                     doc.url, e
    //                                 );
    //                             }
    //                         }
    //                     }
    //                 }
    //                 None => {
    //                     eprintln!("[OSMOzzz Daemon] Watcher arrêté.");
    //                     break;
    //                 }
    //             }
    //         }
    //         _ = tokio::signal::ctrl_c() => {
    //             eprintln!("\n[OSMOzzz Daemon] Arrêt propre (Ctrl+C).");
    //             break;
    //         }
    // ────────────────────────────────────────────────────────────────────────

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                eprintln!("\n[OSMOzzz Daemon] Arrêt propre (Ctrl+C).");
                break;
            }
        }
    }

    Ok(())
}

// ─── Sync générique pour toutes les sources locales ──────────────────────────

async fn sync_docs(vault: &Arc<Vault>, label: &str, docs: Vec<osmozzz_core::Document>) {
    if docs.is_empty() {
        return;
    }

    let blacklist = Blacklist::load();
    let mut indexed = 0;
    for doc in &docs {
        // Skip banned documents (URL ban or source-level ban)
        let source = doc.source.to_string();
        let title  = doc.title.as_deref().unwrap_or("");
        if blacklist.is_banned(&source, &doc.url, title) { continue; }

        if let Ok(true) = vault.exists(&doc.checksum).await { continue; }
        match vault.upsert(doc).await {
            Ok(_) => indexed += 1,
            Err(e) => eprintln!("[OSMOzzz Daemon] {} upsert erreur: {}", label, e),
        }
    }

    if indexed > 0 {
        eprintln!("[OSMOzzz Daemon] {} sync: {} nouveaux documents indexés.", label, indexed);
        if let Err(e) = vault.compact().await {
            eprintln!("[OSMOzzz Daemon] Compact erreur: {}", e);
        }
    }
}

fn default_watch_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs_next::home_dir() {
        for folder in &["Desktop", "Documents", "Downloads"] {
            let p = home.join(folder);
            if p.exists() {
                paths.push(p);
            }
        }
    }
    paths
}
