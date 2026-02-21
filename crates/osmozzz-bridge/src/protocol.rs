use osmozzz_core::SearchResult;
use serde::{Deserialize, Serialize};

/// Incoming request from client (e.g. OpenClaw).
#[derive(Debug, Deserialize)]
#[serde(tag = "method", rename_all = "lowercase")]
pub enum Request {
    Search {
        query: String,
        #[serde(default = "default_limit")]
        limit: usize,
    },
    Status,
    Ping,
}

fn default_limit() -> usize {
    5
}

/// Outgoing response to client.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Response {
    Search { results: Vec<SearchResult> },
    Status { doc_count: usize, status: String },
    Pong { pong: bool },
    Error { error: String },
}
