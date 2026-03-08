use serde::{Deserialize, Serialize};

/// Messages échangés entre deux daemons OSMOzzz via TLS.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// Ping de keepalive
    Ping,
    Pong,

    /// Handshake initial — le connecteur s'identifie
    Hello { peer_id: String, display_name: String },
    /// Réponse au Hello — confirme la connexion
    Welcome { peer_id: String, display_name: String },

    /// Requête de recherche
    Search(SearchRequest),
    /// Résultats de recherche
    SearchResult(SearchResponse),

    /// Demande d'info sur les sources partagées
    GetInfo,
    /// Réponse info
    Info(PeerInfo),

    /// Erreur
    Error { code: String, message: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchRequest {
    pub request_id: String,
    pub query: String,
    pub limit: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    pub request_id: String,
    pub peer_id: String,
    pub peer_name: String,
    pub results: Vec<PeerSearchResult>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerSearchResult {
    pub source: String,
    pub title: Option<String>,
    pub content: String,
    pub score: f32,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub display_name: String,
    pub shared_sources: Vec<String>,
    pub osmozzz_version: String,
}
