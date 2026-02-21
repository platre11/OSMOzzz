use crate::error::Result;
use crate::types::{Document, SearchResult};

/// Trait for data harvesters (Chrome, files, etc.)
pub trait Harvester: Send + Sync {
    /// Harvest documents from the source.
    /// Returns only new documents (not yet indexed).
    fn harvest(&self) -> impl std::future::Future<Output = Result<Vec<Document>>> + Send;
}

/// Trait for the vector embedding and storage engine.
pub trait Embedder: Send + Sync {
    /// Embed and store a document in the vector store.
    fn upsert(&self, doc: &Document) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Search for similar documents.
    fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> impl std::future::Future<Output = Result<Vec<SearchResult>>> + Send;

    /// Check if a document with this checksum already exists.
    fn exists(
        &self,
        checksum: &str,
    ) -> impl std::future::Future<Output = Result<bool>> + Send;

    /// Return the total number of indexed documents.
    fn count(&self) -> impl std::future::Future<Output = Result<usize>> + Send;
}
