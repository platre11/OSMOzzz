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

/// Applique les filtres de confidentialité du propriétaire sur tout résultat
/// sortant vers un peer P2P — même logique que pour son propre Claude local.
///
/// Chaîne :
///   1. PrivacyFilter (privacy.toml) — masque emails, téléphones, clés API
///   2. Aliases (aliases.toml)       — remplace vrais noms par pseudonymes
fn filter_p2p_output(text: &str) -> String {
    use osmozzz_core::filter::{PrivacyConfig, PrivacyFilter};

    // 1. Pare-feu de confidentialité
    let cfg = PrivacyConfig::load();
    let filtered = PrivacyFilter::from_config(&cfg).apply(text);

    // 2. Alias d'identité (vrais noms → pseudonymes)
    let aliases: Vec<(String, String)> = {
        let path = match dirs_next::home_dir() {
            Some(h) => h.join(".osmozzz/aliases.toml"),
            None => return filtered,
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return filtered,
        };
        let t: toml::Value = match content.parse() {
            Ok(v) => v,
            Err(_) => return filtered,
        };
        let mut pairs: Vec<(String, String)> = vec![];
        if let Some(table) = t.as_table() {
            for (_group, entries) in table {
                if let Some(arr) = entries.as_array() {
                    for entry in arr {
                        if let (Some(real), Some(alias)) = (
                            entry.get("real").and_then(|v| v.as_str()),
                            entry.get("alias").and_then(|v| v.as_str()),
                        ) {
                            pairs.push((real.to_string(), alias.to_string()));
                        }
                    }
                }
            }
        }
        // Trier par longueur décroissante pour éviter les substitutions partielles
        pairs.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        pairs
    };

    if aliases.is_empty() {
        return filtered;
    }
    let mut result = filtered;
    for (real, alias) in &aliases {
        result = result.replace(real.as_str(), alias.as_str());
    }
    result
}

pub async fn run(_cfg: Config) -> Result<()> {
    // Créer la queue d'actions locale (Claude → dashboard, isolée du P2P)
    let action_queue = Arc::new(osmozzz_api::ActionQueue::new());
    // Queue dédiée aux approbations P2P (demandes de peers — page Réseau)
    let p2p_action_queue = Arc::new(osmozzz_api::ActionQueue::new());
    // Canal SSE pour notifier le dashboard des changements de permissions P2P
    let (network_tx, _) = tokio::sync::broadcast::channel::<String>(32);

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
            let event_node = node.clone();
            let network_tx_events = network_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = p2p_event_rx.recv().await {
                    match event {
                        P2pEvent::PeerConnected { display_name, peer_id } => {
                            eprintln!("[P2P] {} connecté", display_name);
                            // Notifie le dashboard — mise à jour instantanée du statut
                            let _ = network_tx_events.send(format!("connect:{}", peer_id));
                        }
                        P2pEvent::PermissionsUpdated { peer_id } => {
                            // Notifie le dashboard SSE — mise à jour instantanée côté UI
                            let _ = network_tx_events.send(format!("permissions:{}", peer_id));
                        }
                        P2pEvent::PeerDisconnected { peer_id } => {
                            eprintln!("[P2P] Peer {} déconnecté — reconnexion dans 3s…", &peer_id[..8.min(peer_id.len())]);
                            // Notifie le dashboard — statut déconnecté immédiat
                            let _ = network_tx_events.send(format!("disconnect:{}", peer_id));
                            // Reconnexion immédiate après changement de réseau (WiFi → 4G, etc.)
                            let n = event_node.clone();
                            let pid = peer_id.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                                if let Some(peer) = n.store.get(&pid) {
                                    if let Some(addr) = peer.addresses.first() {
                                        let _ = n.connect_to_peer(addr).await;
                                    }
                                }
                            });
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
                                    Some(Ok(text)) => (Some(filter_p2p_output(&text)), None),
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
                                                    let _ = result_tx.send(Some(filter_p2p_output(&result)));
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

            // Reconnexion automatique toutes les 10s pour les peers connus hors ligne
            let reconnect_node = node.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
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
    let dashboard_network_tx = network_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = osmozzz_api::start_server(dashboard_vault, dashboard_p2p, dashboard_queue, dashboard_p2p_queue, dashboard_network_tx, DASHBOARD_PORT).await {
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
