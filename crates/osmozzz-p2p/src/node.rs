use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use iroh::{Endpoint, EndpointAddr, SecretKey};
use iroh::endpoint::{Connection, RecvStream, SendStream};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

use crate::identity::PeerIdentity;
use crate::permissions::{PeerPermissions, ToolAccessMode};
use crate::protocol::{Message, PeerInfo, SearchResponse, ToolCallResult};
use crate::store::{KnownPeer, PeerStore};
use crate::history::{QueryHistoryEntry, QueryHistoryLog};

/// ALPN protocol identifier — identifie les connexions OSMOzzz P2P.
const ALPN: &[u8] = b"osmozzz/1";

/// Conservé pour la compatibilité API avec daemon.rs.
/// iroh gère le port UDP automatiquement pour optimiser le hole punching.
pub const DEFAULT_P2P_PORT: u16 = 47474;

/// Événement émis par le node P2P vers le reste de l'application.
#[derive(Debug)]
pub enum P2pEvent {
    PeerConnected { peer_id: String, display_name: String },
    PeerDisconnected { peer_id: String },
    QueryReceived { peer_id: String, peer_name: String, query: String },
    /// Mode "Auto" — le daemon doit exécuter le tool et renvoyer le résultat.
    ToolCallAuto {
        peer_id:   String,
        peer_name: String,
        tool_call: crate::protocol::ToolCallRequest,
        result_tx: tokio::sync::oneshot::Sender<ToolCallResult>,
    },
    /// Mode "Approbation requise" — le daemon queue l'action dans le dashboard.
    /// Réponse via `result_tx` : Some(résultat) = approuvé, None = rejeté.
    ToolCallPending {
        peer_id:   String,
        peer_name: String,
        tool_call: crate::protocol::ToolCallRequest,
        result_tx: tokio::sync::oneshot::Sender<Option<String>>,
    },
}

/// Le nœud P2P principal.
///
/// Utilise iroh (QUIC + relay n0.computer) pour fonctionner sur TOUS les réseaux
/// sans configuration manuelle :
/// - Même WiFi → connexion directe locale
/// - WiFis différents → STUN + hole punching
/// - 4G / NAT strict → relay chiffré n0.computer (données illisibles)
pub struct P2pNode {
    pub identity: PeerIdentity,
    pub store: PeerStore,
    pub history: QueryHistoryLog,
    endpoint: Endpoint,
    /// peer_id → channel vers la tâche de connexion active
    connections: RwLock<std::collections::HashMap<String, mpsc::Sender<Message>>>,
    /// request_id → channel de réponse (call_peer_tool → ToolResult)
    pending_tool_calls: RwLock<std::collections::HashMap<String, tokio::sync::oneshot::Sender<ToolCallResult>>>,
    event_tx: mpsc::Sender<P2pEvent>,
}

impl P2pNode {
    pub async fn new(
        display_name: &str,
        _port: u16,
        event_tx: mpsc::Sender<P2pEvent>,
    ) -> Result<Arc<Self>> {
        let identity = PeerIdentity::load_or_create(display_name)
            .context("Chargement identité P2P")?;

        // Dérive la clé iroh depuis notre Ed25519 existante (mêmes 32 octets)
        let secret_key = SecretKey::from_bytes(identity.signing_key.as_bytes());

        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            // ALPN pour accepter les connexions entrantes OSMOzzz
            .alpns(vec![ALPN.to_vec()])
            // relay n0.computer : fallback automatique quand le hole punching échoue
            .bind()
            .await
            .context("Impossible de démarrer l'endpoint iroh QUIC")?;

        info!("[P2P] Nœud iroh démarré — id: {}…", &identity.id[..16]);

        Ok(Arc::new(Self {
            identity,
            store: PeerStore::new()?,
            history: QueryHistoryLog::new()?,
            endpoint,
            connections: RwLock::new(std::collections::HashMap::new()),
            pending_tool_calls: RwLock::new(std::collections::HashMap::new()),
            event_tx,
        }))
    }

    /// Démarre le serveur iroh — accepte les connexions QUIC entrantes.
    pub async fn start_server(self: Arc<Self>) -> Result<()> {
        let node = self.clone();
        tokio::spawn(async move {
            info!("[P2P] En écoute QUIC (relay n0.computer actif)");
            while let Some(incoming) = node.endpoint.accept().await {
                let node = node.clone();
                tokio::spawn(async move {
                    match incoming.accept() {
                        Ok(accepting) => match accepting.await {
                            Ok(conn) => {
                                if let Err(e) = node.handle_incoming(conn).await {
                                    warn!("[P2P] Erreur connexion entrante : {}", e);
                                }
                            }
                            Err(e) => warn!("[P2P] Connexion invalide : {}", e),
                        },
                        Err(e) => warn!("[P2P] Connexion refusée : {}", e),
                    }
                });
            }
        });
        Ok(())
    }

    /// Gère une connexion QUIC entrante.
    /// L'identité du peer est vérifiée cryptographiquement via TLS (conn.remote_id()).
    async fn handle_incoming(self: Arc<Self>, conn: Connection) -> Result<()> {
        let peer_id = hex::encode(conn.remote_id().as_bytes());
        let (send, recv) = conn.accept_bi().await
            .context("Impossible d'ouvrir le stream QUIC entrant")?;
        self.handle_stream(send, recv, peer_id, true).await
    }

    /// Établit une connexion sortante vers un peer via son adresse iroh (base64 JSON).
    /// L'adresse contient : EndpointId + adresses directes + URL relay.
    pub async fn connect_to_peer(self: Arc<Self>, addr_b64: &str) -> Result<()> {
        let json = URL_SAFE_NO_PAD.decode(addr_b64)
            .context("Décodage base64 du ticket iroh invalide")?;
        let addr: EndpointAddr = serde_json::from_slice(&json)
            .context("Ticket iroh JSON invalide")?;
        let peer_id = hex::encode(addr.id.as_bytes());

        let conn = self.endpoint.connect(addr, ALPN).await
            .context(format!("Connexion au peer {}… impossible", &peer_id[..16.min(peer_id.len())]))?;

        info!("[P2P] Connecté à peer {}…", &peer_id[..16]);

        let (send, recv) = conn.open_bi().await
            .context("Impossible d'ouvrir le stream QUIC sortant")?;

        let node = self.clone();
        tokio::spawn(async move {
            if let Err(e) = node.handle_stream(send, recv, peer_id, false).await {
                warn!("[P2P] Erreur connexion sortante : {}", e);
            }
        });

        Ok(())
    }

    /// Gère les échanges de messages sur un stream QUIC bidirectionnel.
    ///
    /// - `known_peer_id` = peer_id déjà connu (issu du TLS)
    /// - `is_incoming = true` → on attend Hello avant de faire quoi que ce soit
    /// - `is_incoming = false` → on envoie Hello en premier
    async fn handle_stream(
        self: Arc<Self>,
        mut send: SendStream,
        recv: RecvStream,
        known_peer_id: String,
        is_incoming: bool,
    ) -> Result<()> {
        let mut reader = BufReader::new(recv);
        let mut line = String::new();
        let peer_id = known_peer_id;
        let mut peer_name = String::from("Unknown");
        let (tx, mut rx) = mpsc::channel::<Message>(32);

        // Connexion sortante : on envoie Hello en premier pour partager le display name
        if !is_incoming {
            send_msg(&mut send, &Message::Hello {
                peer_id: self.identity.id.clone(),
                display_name: whoami_name(),
            }).await?;

            line.clear();
            reader.read_line(&mut line).await?;
            match serde_json::from_str::<Message>(line.trim())? {
                Message::Welcome { display_name, .. } => {
                    peer_name = display_name.clone();
                    self.connections.write().await.insert(peer_id.clone(), tx.clone());
                    self.store.set_connected(&peer_id, true).ok();
                    let _ = self.event_tx.send(P2pEvent::PeerConnected {
                        peer_id: peer_id.clone(),
                        display_name,
                    }).await;
                    // Envoyer nos permissions au peer juste après le handshake
                    let my_perms = self.get_permissions(&peer_id);
                    let tool_map: std::collections::HashMap<String, String> = my_perms.tool_permissions.iter()
                        .map(|(k, v)| (k.clone(), format!("{:?}", v).to_lowercase()))
                        .collect();
                    let _ = send_msg(&mut send, &crate::protocol::Message::PermissionsSync {
                        allowed_sources: my_perms.allowed_source_names(),
                        tool_permissions: tool_map,
                    }).await;
                }
                Message::Error { message, .. } => {
                    return Err(anyhow::anyhow!("Peer a refusé : {}", message));
                }
                _ => return Err(anyhow::anyhow!("Réponse inattendue au Hello")),
            }
        }

        // Boucle principale de messages
        loop {
            line.clear();
            tokio::select! {
                n = reader.read_line(&mut line) => {
                    if n? == 0 { break; }
                    let msg: Message = match serde_json::from_str(line.trim()) {
                        Ok(m) => m,
                        Err(e) => { warn!("[P2P] Message invalide : {}", e); continue; }
                    };
                    match msg {
                        // Connexion entrante : on reçoit Hello et on répond Welcome
                        // L'identité est déjà vérifiée par TLS — Hello sert uniquement à
                        // échanger le display name et à vérifier l'autorisation dans le store.
                        Message::Hello { peer_id: hello_id, display_name } if is_incoming => {
                            // Vérification autorisation dans le store
                            if !self.is_peer_authorized(&peer_id) {
                                let _ = send_msg(&mut send, &Message::Error {
                                    code: "UNAUTHORIZED".into(),
                                    message: "Peer non autorisé. Utilise osmozzz://invite/ d'abord.".into(),
                                }).await;
                                break;
                            }
                            // Optionnel : log si le hello_id ne correspond pas au peer TLS
                            if hello_id != peer_id {
                                warn!("[P2P] Hello peer_id mismatch : {} vs {}", &hello_id[..8], &peer_id[..8]);
                            }
                            peer_name = display_name.clone();
                            self.connections.write().await.insert(peer_id.clone(), tx.clone());
                            self.store.set_connected(&peer_id, true).ok();
                            send_msg(&mut send, &Message::Welcome {
                                peer_id: self.identity.id.clone(),
                                display_name: whoami_name(),
                            }).await?;
                            let _ = self.event_tx.send(P2pEvent::PeerConnected {
                                peer_id: peer_id.clone(),
                                display_name,
                            }).await;
                            // Envoyer nos permissions au peer juste après le Welcome
                            let my_perms = self.get_permissions(&peer_id);
                            let tool_map: std::collections::HashMap<String, String> = my_perms.tool_permissions.iter()
                                .map(|(k, v)| (k.clone(), format!("{:?}", v).to_lowercase()))
                                .collect();
                            let _ = send_msg(&mut send, &Message::PermissionsSync {
                                allowed_sources: my_perms.allowed_source_names(),
                                tool_permissions: tool_map,
                            }).await;
                        }

                        Message::PermissionsSync { allowed_sources, tool_permissions } => {
                            use crate::permissions::{PeerPermissions, SharedSource, ToolAccessMode};
                            let sources: Vec<SharedSource> = allowed_sources.iter()
                                .filter_map(|s| match s.as_str() {
                                    "email" => Some(SharedSource::Email),
                                    "imessage" => Some(SharedSource::IMessage),
                                    "notes" => Some(SharedSource::Notes),
                                    "calendar" => Some(SharedSource::Calendar),
                                    "terminal" => Some(SharedSource::Terminal),
                                    "file" => Some(SharedSource::File),
                                    "notion" => Some(SharedSource::Notion),
                                    "github" => Some(SharedSource::Github),
                                    "linear" => Some(SharedSource::Linear),
                                    "jira" => Some(SharedSource::Jira),
                                    "slack" => Some(SharedSource::Slack),
                                    "trello" => Some(SharedSource::Trello),
                                    "todoist" => Some(SharedSource::Todoist),
                                    "gitlab" => Some(SharedSource::Gitlab),
                                    "airtable" => Some(SharedSource::Airtable),
                                    "obsidian" => Some(SharedSource::Obsidian),
                                    _ => None,
                                }).collect();
                            let tool_perms: std::collections::HashMap<String, ToolAccessMode> = tool_permissions.iter()
                                .map(|(k, v)| {
                                    let mode = match v.as_str() {
                                        "require" => ToolAccessMode::Require,
                                        "disabled" => ToolAccessMode::Disabled,
                                        _ => ToolAccessMode::Auto,
                                    };
                                    (k.clone(), mode)
                                }).collect();
                            let granted = PeerPermissions {
                                allowed_sources: sources,
                                max_results_per_query: 10,
                                tool_permissions: tool_perms,
                            };
                            self.store.update_peer_granted(&peer_id, granted).ok();
                            info!("[P2P] Permissions reçues de {}", &peer_id[..8.min(peer_id.len())]);
                        }

                        Message::Search(req) => {
                            let _ = self.event_tx.send(P2pEvent::QueryReceived {
                                peer_id: peer_id.clone(),
                                peer_name: peer_name.clone(),
                                query: req.query.clone(),
                            }).await;

                            let perms = self.get_permissions(&peer_id);
                            let results = self.execute_search(&req.query, req.limit, &perms).await;

                            self.history.append(&QueryHistoryEntry {
                                ts: chrono::Utc::now().timestamp(),
                                peer_id: peer_id.clone(),
                                peer_name: peer_name.clone(),
                                query: req.query.clone(),
                                results_count: results.len(),
                                blocked: false,
                                kind: "search".to_string(),
                            }).ok();

                            send_msg(&mut send, &Message::SearchResult(SearchResponse {
                                request_id: req.request_id,
                                peer_id: self.identity.id.clone(),
                                peer_name: whoami_name(),
                                results,
                            })).await?;
                        }

                        Message::GetInfo => {
                            let perms = self.get_permissions(&peer_id);
                            send_msg(&mut send, &Message::Info(PeerInfo {
                                peer_id: self.identity.id.clone(),
                                display_name: whoami_name(),
                                shared_sources: perms.allowed_source_names(),
                                osmozzz_version: "0.1.0".into(),
                            })).await?;
                        }

                        Message::ToolCall(req) => {
                            let perms = self.get_permissions(&peer_id);
                            // Connecteur = préfixe du tool_name (ex: "github_list_issues" → "github")
                            let connector = req.tool_name.split('_').next().unwrap_or(&req.tool_name);
                            let mode = perms.tool_permissions.get(connector)
                                .cloned()
                                .unwrap_or(ToolAccessMode::Auto);

                            // Log l'appel dans query_history.jsonl (séparé de audit.jsonl)
                            self.history.append(&QueryHistoryEntry {
                                ts: chrono::Utc::now().timestamp(),
                                peer_id: peer_id.clone(),
                                peer_name: peer_name.clone(),
                                query: req.tool_name.clone(),
                                results_count: 0,
                                blocked: mode == ToolAccessMode::Disabled,
                                kind: "tool_call".to_string(),
                            }).ok();

                            let response = match mode {
                                ToolAccessMode::Disabled => {
                                    Message::ToolResult(ToolCallResult {
                                        request_id: req.request_id,
                                        peer_id: self.identity.id.clone(),
                                        peer_name: whoami_name(),
                                        tool_name: req.tool_name,
                                        result: None,
                                        error: Some("Accès refusé par le propriétaire des données".into()),
                                    })
                                }
                                ToolAccessMode::Require => {
                                    // Envoie l'événement au daemon pour passer par la file d'approbation
                                    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
                                    let _ = self.event_tx.send(P2pEvent::ToolCallPending {
                                        peer_id: peer_id.clone(),
                                        peer_name: peer_name.clone(),
                                        tool_call: req.clone(),
                                        result_tx,
                                    }).await;

                                    // Attend la décision (5 min max)
                                    match tokio::time::timeout(
                                        tokio::time::Duration::from_secs(300),
                                        result_rx,
                                    ).await {
                                        Ok(Ok(Some(result))) => Message::ToolResult(ToolCallResult {
                                            request_id: req.request_id,
                                            peer_id: self.identity.id.clone(),
                                            peer_name: whoami_name(),
                                            tool_name: req.tool_name,
                                            result: Some(result),
                                            error: None,
                                        }),
                                        Ok(Ok(None)) => Message::ToolResult(ToolCallResult {
                                            request_id: req.request_id,
                                            peer_id: self.identity.id.clone(),
                                            peer_name: whoami_name(),
                                            tool_name: req.tool_name,
                                            result: None,
                                            error: Some("Action rejetée par le propriétaire".into()),
                                        }),
                                        _ => Message::ToolResult(ToolCallResult {
                                            request_id: req.request_id,
                                            peer_id: self.identity.id.clone(),
                                            peer_name: whoami_name(),
                                            tool_name: req.tool_name,
                                            result: None,
                                            error: Some("Délai d'approbation dépassé (5 min)".into()),
                                        }),
                                    }
                                }
                                ToolAccessMode::Auto => {
                                    // Auto : le daemon exécute le tool réel via les connecteurs
                                    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
                                    let _ = self.event_tx.send(P2pEvent::ToolCallAuto {
                                        peer_id: peer_id.clone(),
                                        peer_name: peer_name.clone(),
                                        tool_call: req.clone(),
                                        result_tx,
                                    }).await;

                                    match tokio::time::timeout(
                                        tokio::time::Duration::from_secs(30),
                                        result_rx,
                                    ).await {
                                        Ok(Ok(result)) => Message::ToolResult(result),
                                        _ => Message::ToolResult(ToolCallResult {
                                            request_id: req.request_id,
                                            peer_id: self.identity.id.clone(),
                                            peer_name: whoami_name(),
                                            tool_name: req.tool_name,
                                            result: None,
                                            error: Some("Délai d'exécution dépassé (30s)".into()),
                                        }),
                                    }
                                }
                            };
                            send_msg(&mut send, &response).await?;
                        }

                        // ToolResult : route vers call_peer_tool() en attente
                        Message::ToolResult(result) => {
                            let mut pending = self.pending_tool_calls.write().await;
                            if let Some(tx) = pending.remove(&result.request_id) {
                                let _ = tx.send(result);
                            }
                        }

                        Message::Ping => { send_msg(&mut send, &Message::Pong).await?; }

                        _ => {}
                    }
                }

                Some(msg) = rx.recv() => {
                    send_msg(&mut send, &msg).await?;
                }
            }
        }

        // Nettoyage à la déconnexion
        if !peer_id.is_empty() {
            self.connections.write().await.remove(&peer_id);
            self.store.set_connected(&peer_id, false).ok();
            let _ = self.event_tx.send(P2pEvent::PeerDisconnected {
                peer_id: peer_id.clone(),
            }).await;
        }
        let _ = send.finish();

        Ok(())
    }

    /// Envoie une requête de recherche à tous les peers connectés (ou un seul).
    pub async fn search_peers(
        &self,
        query: &str,
        limit: usize,
        peer_id_filter: Option<&str>,
    ) -> Vec<SearchResponse> {
        use uuid::Uuid;
        let connections = self.connections.read().await;
        let mut handles = vec![];

        for (pid, tx) in connections.iter() {
            if let Some(f) = peer_id_filter { if pid != f { continue; } }
            let req = Message::Search(crate::protocol::SearchRequest {
                request_id: Uuid::new_v4().to_string(),
                query: query.to_string(),
                limit,
            });
            let tx = tx.clone();
            let pid = pid.clone();
            handles.push(tokio::spawn(async move {
                if tx.send(req).await.is_err() {
                    warn!("[P2P] Impossible d'envoyer requête à {}…", &pid[..16.min(pid.len())]);
                }
            }));
        }

        for h in handles { let _ = h.await; }
        vec![]
    }

    async fn execute_search(
        &self,
        query: &str,
        limit: usize,
        perms: &PeerPermissions,
    ) -> Vec<crate::protocol::PeerSearchResult> {
        let _ = (query, limit, perms);
        vec![]
    }

    fn is_peer_authorized(&self, peer_id: &str) -> bool {
        self.store.get(peer_id).is_some()
    }

    fn get_permissions(&self, peer_id: &str) -> PeerPermissions {
        self.store.get(peer_id).map(|p| p.permissions).unwrap_or_default()
    }

    /// Pousse immédiatement les nouvelles permissions vers un peer connecté.
    /// Appelé dès que l'utilisateur modifie les permissions dans le dashboard,
    /// sans attendre une reconnexion — critique pour la sécurité.
    pub async fn push_permissions_to_peer(&self, peer_id: &str) {
        let connections = self.connections.read().await;
        if let Some(tx) = connections.get(peer_id) {
            let my_perms = self.get_permissions(peer_id);
            let tool_map: std::collections::HashMap<String, String> = my_perms
                .tool_permissions
                .iter()
                .map(|(k, v)| (k.clone(), format!("{:?}", v).to_lowercase()))
                .collect();
            let msg = Message::PermissionsSync {
                allowed_sources: my_perms.allowed_source_names(),
                tool_permissions: tool_map,
            };
            if tx.send(msg).await.is_err() {
                warn!("[P2P] push_permissions_to_peer: channel fermé pour {}", &peer_id[..16.min(peer_id.len())]);
            }
        }
    }

    /// Génère un lien d'invitation iroh — fonctionne sur tous les réseaux.
    /// Contient : EndpointId + adresses directes + URL relay n0.computer.
    /// Format : osmozzz://invite/<base64_json_EndpointAddr>
    pub async fn generate_invite_link(&self) -> Result<String> {
        let addr = self.endpoint.addr();
        let json = serde_json::to_vec(&addr)
            .context("Sérialisation EndpointAddr")?;
        let encoded = URL_SAFE_NO_PAD.encode(&json);
        Ok(format!("osmozzz://invite/{}", encoded))
    }

    /// Parse un lien d'invitation iroh et enregistre le peer dans le store.
    pub fn accept_invite(&self, link: &str, display_name: &str) -> Result<KnownPeer> {
        let encoded = link.trim_start_matches("osmozzz://invite/");
        let json = URL_SAFE_NO_PAD.decode(encoded)
            .context("Décodage base64 du lien d'invitation invalide")?;
        let addr: EndpointAddr = serde_json::from_slice(&json)
            .context("Lien d'invitation JSON invalide")?;
        let peer_id = hex::encode(addr.id.as_bytes());

        let peer = KnownPeer {
            peer_id: peer_id.clone(),
            display_name: display_name.to_string(),
            // Stocke le ticket base64 comme adresse pour connect_to_peer
            addresses: vec![encoded.to_string()],
            public_key_hex: peer_id.clone(),
            permissions: PeerPermissions::default(),
            peer_granted_to_me: None,
            connected: false,
            last_seen: None,
        };

        self.store.upsert(peer.clone())?;
        info!("[P2P] Peer {} enregistré via invite iroh", display_name);

        Ok(peer)
    }

    pub async fn connected_peer_ids(&self) -> Vec<String> {
        self.connections.read().await.keys().cloned().collect()
    }

    /// Retourne la liste des peers connectés avec leur display_name.
    pub async fn connected_peers_list(&self) -> Vec<(String, String)> {
        let connections = self.connections.read().await;
        let store_peers = self.store.all();
        connections.keys().map(|peer_id| {
            let name = store_peers.iter()
                .find(|p| &p.peer_id == peer_id)
                .map(|p| p.display_name.clone())
                .unwrap_or_else(|| format!("{}…", &peer_id[..8.min(peer_id.len())]));
            (peer_id.clone(), name)
        }).collect()
    }

    /// Appelle un tool MCP sur un peer distant et attend la réponse.
    /// Respecte le mode d'accès configuré (Auto / Approbation / Désactivé) côté peer.
    /// Timeout : 120 secondes (laisse le temps à une approbation manuelle).
    pub async fn call_peer_tool(
        &self,
        peer_id: &str,
        tool_name: &str,
        params: serde_json::Value,
    ) -> Result<ToolCallResult> {
        use uuid::Uuid;

        let request_id = Uuid::new_v4().to_string();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel::<ToolCallResult>();

        {
            let connections = self.connections.read().await;
            let tx = connections.get(peer_id)
                .ok_or_else(|| anyhow::anyhow!(
                    "Peer {} non connecté — vérifie la page Réseau",
                    &peer_id[..16.min(peer_id.len())]
                ))?;

            self.pending_tool_calls.write().await.insert(request_id.clone(), result_tx);

            let msg = Message::ToolCall(crate::protocol::ToolCallRequest {
                request_id: request_id.clone(),
                tool_name: tool_name.to_string(),
                params,
            });

            if tx.send(msg).await.is_err() {
                self.pending_tool_calls.write().await.remove(&request_id);
                return Err(anyhow::anyhow!("Connexion au peer perdue"));
            }
        }

        match tokio::time::timeout(
            tokio::time::Duration::from_secs(120),
            result_rx,
        ).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => {
                self.pending_tool_calls.write().await.remove(&request_id);
                Err(anyhow::anyhow!("Channel de réponse fermé"))
            }
            Err(_) => {
                self.pending_tool_calls.write().await.remove(&request_id);
                Err(anyhow::anyhow!("Délai dépassé (120s) — l'approbation n'a pas eu lieu"))
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn send_msg(
    writer: &mut SendStream,
    msg: &Message,
) -> Result<()> {
    let mut json = serde_json::to_string(msg)?;
    json.push('\n');
    writer.write_all(json.as_bytes()).await?;
    Ok(())
}

fn whoami_name() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "OSMOzzz".to_string())
}
