use std::path::PathBuf;

use osmozzz_core::{Document, Embedder, Result, SearchResult};
use tracing::info;

use crate::embedder::OnnxEmbedder;
use crate::store::VectorStore;

/// The Vault combines the ONNX embedder and LanceDB vector store.
/// This is the primary interface for the embedder crate.
pub struct Vault {
    embedder: OnnxEmbedder,
    store: VectorStore,
}

impl Vault {
    /// Initialize the vault with model and DB paths.
    pub async fn open(
        model_path: &PathBuf,
        tokenizer_path: &PathBuf,
        db_path: &str,
    ) -> Result<Self> {
        let embedder = OnnxEmbedder::load(model_path, tokenizer_path)?;
        let store = VectorStore::open(db_path).await?;

        info!("Vault initialized: model={}, db={}", model_path.display(), db_path);
        Ok(Self { embedder, store })
    }
}

impl Embedder for Vault {
    async fn upsert(&self, doc: &Document) -> Result<()> {
        let embedding = self.embedder.embed(&doc.content)?;
        self.store.upsert(doc, embedding).await
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let embedding = self.embedder.embed(query)?;
        self.store.search(embedding, limit).await
    }

    async fn exists(&self, checksum: &str) -> Result<bool> {
        self.store.exists(checksum).await
    }

    async fn count(&self) -> Result<usize> {
        self.store.count().await
    }
}
