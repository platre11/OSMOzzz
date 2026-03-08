use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use iroh::{Endpoint, EndpointAddr, SecretKey};
use iroh::endpoint::{Connection, RecvStream, SendStream};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

use crate::identity::PeerIdentity;
use crate::permissions::PeerPermissions;
use crate::protocol::{Message, PeerInfo, SearchResponse};
use crate::store::{KnownPeer, PeerStore};
use crate::history::{QueryHistoryEntry, QueryHistoryLog};

/// ALPN protocol identifier — identifie les connexions OSMOzzz P2P.
const ALPN: &[u8] = b"osmozzz/1";

/// Conservé pour la compatibilité API avec daemon.rs.
/// iroh gère le port UDP automatiquement pour optimiser le hole punching.
pub const DEFAULT_P2P_PORT: u16 = 47474;

/// Événement émis par le node P2P vers le reste de l'application.
#[derive(Debug, Clone)]
pub enum P2pEvent {
    PeerConnected { peer_id: String, display_name: String },
    PeerDisconnected { peer_id: String },
    QueryReceived { peer_id: String, peer_name: String, query: String },
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
