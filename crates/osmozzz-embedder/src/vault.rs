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

impl Vault {
    /// Expose l'embedding brut pour le scoring à la volée (fetch intelligent).
    pub fn embed_raw(&self, text: &str) -> osmozzz_core::Result<Vec<f32>> {
        self.embedder.embed(text)
    }

    /// Compact the vector store: merge all fragment files and prune old versions.
    pub async fn compact(&self) -> osmozzz_core::Result<()> {
        self.store.compact().await
    }

    pub async fn count_source(&self, source: &str) -> osmozzz_core::Result<usize> {
        self.store.count_source(source).await
    }

    pub async fn delete_by_source(&self, source: &str) -> osmozzz_core::Result<()> {
        self.store.delete_by_source(source).await
    }

    pub async fn get_full_content_by_url(&self, url: &str) -> osmozzz_core::Result<Option<(Option<String>, String)>> {
        self.store.get_full_content_by_url(url).await
    }

    pub async fn get_emails_by_sender_and_date(&self, pattern: &str, from_ts: i64, to_ts: i64, limit: usize) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        self.store.get_emails_by_sender_and_date(pattern, from_ts, to_ts, limit).await
    }

    pub async fn get_emails_by_date(&self, from_ts: i64, to_ts: i64, limit: usize) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        self.store.get_emails_by_date(from_ts, to_ts, limit).await
    }

    pub async fn get_emails_by_sender(&self, pattern: &str, limit: usize) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        self.store.get_emails_by_sender(pattern, limit).await
    }

    pub async fn recent_emails_full(&self, limit: usize) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        self.store.recent_emails_full(limit).await
    }

    /// Keyword scan across ALL email content (from + subject + body).
    /// Same philosophy as filesystem find_file: no ONNX, pure string match.
    pub async fn search_emails_by_keyword(&self, keyword: &str, limit: usize) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        self.store.search_emails_by_keyword(keyword, limit).await
    }

    pub async fn recent_emails(&self, limit: usize) -> osmozzz_core::Result<Vec<osmozzz_core::SearchResult>> {
        self.store.recent_by_source("email", limit).await
    }

    pub async fn search_filtered(
        &self,
        query: &str,
        limit: usize,
        source_filter: Option<&str>,
    ) -> osmozzz_core::Result<Vec<osmozzz_core::SearchResult>> {
        let embedding = self.embedder.embed(query)?;
        self.store.search_filtered(embedding, limit, source_filter).await
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
