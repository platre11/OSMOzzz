use std::path::PathBuf;

use osmozzz_core::{Document, Embedder, Result, SearchResult};
use tracing::info;

use crate::blacklist::Blacklist;
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

    pub async fn get_docs_info_by_urls(&self, urls: &[String]) -> osmozzz_core::Result<Vec<(String, String, Option<String>, String)>> {
        self.store.get_docs_info_by_urls(urls).await
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

    /// Generic keyword search filtered by source (imessage, notes, terminal, calendar, safari…).
    pub async fn search_by_keyword_source(
        &self,
        keyword: &str,
        limit: usize,
        source: &str,
    ) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        self.store.search_by_keyword_source(keyword, limit, source).await
    }

    /// Keyword scan across ALL sources.
    pub async fn search_all_by_keyword(
        &self,
        keyword: &str,
        limit: usize,
    ) -> osmozzz_core::Result<Vec<(Option<String>, String, String, String)>> {
        let sources = ["email", "chrome", "file", "imessage", "safari", "notes", "terminal", "calendar", "notion", "github", "linear", "jira", "slack", "trello", "todoist", "gitlab", "airtable", "obsidian"];
        let per_source = (limit / sources.len()).max(5);
        let mut all = Vec::new();
        for src in &sources {
            let results = self.store.search_by_keyword_source(keyword, per_source, src).await.unwrap_or_default();
            for (title, content, url) in results {
                all.push((title, content, url, src.to_string()));
            }
        }
        all.truncate(limit);
        Ok(all)
    }

    /// Grouped keyword search: top `per_source` most recent results per source.
    /// Returns only non-empty sources. (ts, title, url, content)
    pub async fn search_grouped_by_keyword(
        &self,
        keyword: &str,
        per_source: usize,
    ) -> osmozzz_core::Result<std::collections::HashMap<String, Vec<(i64, Option<String>, String, String)>>> {
        let sources = ["email", "chrome", "file", "imessage", "safari", "notes", "terminal", "calendar", "notion", "github", "linear", "jira", "slack", "trello", "todoist", "gitlab", "airtable", "obsidian"];
        let bl = Blacklist::load();
        let mut grouped = std::collections::HashMap::new();
        for src in &sources {
            let results = self.store.search_by_keyword_source_dated(keyword, per_source + 50, src).await.unwrap_or_default();
            let filtered: Vec<_> = results.into_iter()
                .filter(|(_ts, title, url, content)| {
                    !bl.is_result_banned(src, url, title.as_deref().unwrap_or(""), content)
                })
                .take(per_source)
                .collect();
            if !filtered.is_empty() {
                grouped.insert(src.to_string(), filtered);
            }
        }
        Ok(grouped)
    }

    // ─── Blacklist ────────────────────────────────────────────────────────────

    pub fn load_blacklist(&self) -> Blacklist {
        Blacklist::load()
    }

    /// Ban: only adds to blacklist.toml — data stays in LanceDB for instant unban.
    pub async fn ban_url(&self, url: &str) -> osmozzz_core::Result<()> {
        let mut bl = Blacklist::load();
        bl.ban_url(url);
        bl.save().map_err(|e| osmozzz_core::OsmozzError::Storage(e.to_string()))
    }

    pub async fn ban_source_item(&self, source: &str, identifier: &str) -> osmozzz_core::Result<()> {
        let mut bl = Blacklist::load();
        bl.ban_source_item(source, identifier);
        bl.save().map_err(|e| osmozzz_core::OsmozzError::Storage(e.to_string()))
    }

    pub async fn unban_url(&self, url: &str) -> osmozzz_core::Result<()> {
        let mut bl = Blacklist::load();
        bl.unban_url(url);
        bl.save().map_err(|e| osmozzz_core::OsmozzError::Storage(e.to_string()))
    }

    pub async fn unban_source_item(&self, source: &str, identifier: &str) -> osmozzz_core::Result<()> {
        let mut bl = Blacklist::load();
        bl.unban_source_item(source, identifier);
        bl.save().map_err(|e| osmozzz_core::OsmozzError::Storage(e.to_string()))
    }

    pub fn get_blacklist(&self) -> Blacklist {
        Blacklist::load()
    }

    // ─── Métriques performance ────────────────────────────────────────────────

    /// DB size on disk in bytes.
    pub fn db_disk_bytes(&self) -> u64 {
        self.store.disk_bytes()
    }

    /// Current process RSS memory in MB (macOS only, best-effort).
    pub fn process_rss_mb() -> Option<u64> {
        let pid = std::process::id();
        let out = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        let kb: u64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
        Some(kb / 1024)
    }

    pub async fn get_imessage_contacts(&self) -> osmozzz_core::Result<Vec<(String, String, i64, usize)>> {
        self.store.get_imessage_contacts().await
    }

    pub async fn get_imessage_conversation(&self, phone: &str, limit: usize) -> osmozzz_core::Result<Vec<(i64, bool, String)>> {
        self.store.get_imessage_conversation(phone, limit).await
    }

    pub async fn recent_emails(&self, limit: usize) -> osmozzz_core::Result<Vec<osmozzz_core::SearchResult>> {
        self.store.recent_by_source("email", limit).await
    }

    pub async fn recent_by_source(&self, source: &str, limit: usize) -> osmozzz_core::Result<Vec<osmozzz_core::SearchResult>> {
        let bl = Blacklist::load();
        let results = self.store.recent_by_source(source, limit + 500).await?;
        Ok(results.into_iter()
            .filter(|r| !bl.is_result_banned(&r.source, &r.url, r.title.as_deref().unwrap_or(""), &r.content))
            .take(limit)
            .collect())
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
