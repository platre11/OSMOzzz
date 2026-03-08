use std::sync::Arc;
use osmozzz_embedder::Vault;
use osmozzz_p2p::P2pNode;

#[derive(Clone, Default, serde::Serialize)]
pub struct IndexProgress {
    pub running: bool,
    pub total: usize,
    pub processed: usize,
    pub indexed: usize,
    pub skipped: usize,
    pub current_file: String,
}

#[derive(Clone)]
pub struct AppState {
    pub vault: Arc<Vault>,
    pub p2p: Option<Arc<P2pNode>>,
    pub index_progress: Arc<std::sync::Mutex<IndexProgress>>,
}
