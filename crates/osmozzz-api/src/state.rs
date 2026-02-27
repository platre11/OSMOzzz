use std::sync::Arc;
use osmozzz_embedder::Vault;

#[derive(Clone)]
pub struct AppState {
    pub vault: Arc<Vault>,
}
