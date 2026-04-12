/// Service watcher autonome — indépendant du MCP et de l'IA.
///
/// Lance le serveur REST (dashboard) + P2P en arrière-plan.
/// Démarrage instantané — pas de chargement de modèle ONNX.
///
/// Lancement : osmozzz daemon
/// Arrêt propre : Ctrl+C
use std::sync::Arc;

use anyhow::Result;

use osmozzz_api;
use osmozzz_p2p::{P2pNode, ToolCallResult, node::{P2pEvent, DEFAULT_P2P_PORT}};

use crate::config::Config;

const DASHBOARD_PORT: u16 = 7878;

pub async fn run(_cfg: Config) -> Result<()> {
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

            // Reconnexion automatique toutes les 30s pour les peers connus hors ligne
            let reconnect_node = node.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    let connected = reconnect_node.connected_peer_ids().await;
                    let known = reconnect_node.store.all();
                    for peer in known {
                        if !connected.contains(&peer.peer_id) {
                            if let Some(addr) = peer.addresses.first() {
                                let n = reconnect_node.clone();
                                let a = addr.clone();
                                tokio::spawn(async move {
                                    let _ = n.connect_to_peer(&a).await;
                                });
                            }
                        }
                    }
                }
            });

            Some(node)
        }
        Err(e) => {
            eprintln!("[OSMOzzz Daemon] P2P désactivé : {}", e);
            None
        }
    };

    // Démarrer le serveur HTTP du dashboard
    let vault = Arc::new(osmozzz_embedder::Vault::open(
        &std::path::PathBuf::new(),
        &std::path::PathBuf::new(),
        "",
    ).await.unwrap());
    let dashboard_vault = Arc::clone(&vault);
    let dashboard_p2p = p2p_node.clone();
    let dashboard_queue = Arc::clone(&action_queue);
    let dashboard_p2p_queue = Arc::clone(&p2p_action_queue);
    tokio::spawn(async move {
        if let Err(e) = osmozzz_api::start_server(dashboard_vault, dashboard_p2p, dashboard_queue, dashboard_p2p_queue, DASHBOARD_PORT).await {
            eprintln!("[OSMOzzz Daemon] Dashboard erreur: {}", e);
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

    eprintln!("[OSMOzzz Daemon] En écoute... (Ctrl+C pour arrêter)");

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
