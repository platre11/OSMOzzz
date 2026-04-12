use serde::{Deserialize, Serialize};

/// Messages échangés entre deux daemons OSMOzzz via QUIC/iroh.
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

    /// Requête de recherche sémantique (sources locales indexées)
    Search(SearchRequest),
    /// Résultats de recherche
    SearchResult(SearchResponse),

    /// Appel d'un tool MCP distant (connecteur cloud ou action)
    ToolCall(ToolCallRequest),
    /// Résultat d'un appel tool MCP
    ToolResult(ToolCallResult),

    /// Demande d'info sur les sources partagées
    GetInfo,
    /// Réponse info
    Info(PeerInfo),

    /// Synchronisation des permissions — envoyé juste après Welcome/Hello
    /// pour informer le peer de ce qu'on lui autorise sur notre machine.
    PermissionsSync {
        /// Sources autorisées (ex: ["email", "notes", "github"])
        allowed_sources: Vec<String>,
        /// Tools autorisés avec leur mode (ex: {"linear": "auto", "jira": "require"})
        tool_permissions: std::collections::HashMap<String, String>,
    },

    /// Erreur protocolaire
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

/// Appel d'un tool MCP hébergé sur le peer distant.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallRequest {
    /// UUID de la requête — utilisé pour matcher la réponse
    pub request_id: String,
    /// Nom du tool tel qu'il est déclaré dans OSMOzzz (ex: "linear_list_issues")
    pub tool_name: String,
    /// Paramètres JSON du tool
    pub params: serde_json::Value,
}

/// Réponse à un appel tool MCP.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallResult {
    pub request_id: String,
    pub peer_id: String,
    pub peer_name: String,
    pub tool_name: String,
    /// Résultat sérialisé si succès
    pub result: Option<String>,
    /// Message d'erreur si échec
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub display_name: String,
    pub shared_sources: Vec<String>,
    pub osmozzz_version: String,
}
