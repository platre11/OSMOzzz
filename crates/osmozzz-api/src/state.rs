use std::sync::Arc;
use osmozzz_embedder::Vault;
use osmozzz_p2p::P2pNode;

#[derive(Clone)]
pub struct AppState {
    pub vault: Arc<Vault>,
    pub p2p: Option<Arc<P2pNode>>,
}
