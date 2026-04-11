/// osmozzz-embedder stub — ONNX/LanceDB removed.
/// All local indexing replaced by direct MCP API connectors.
/// The Vault type is kept as a no-op placeholder so other crates
/// compile without changes to their imports.

pub mod blacklist;

pub use blacklist::Blacklist;

use osmozzz_core::{Document, OsmozzError, SearchResult};
use std::collections::HashMap;

// ─── Vault (stub — no-op) ───────────────────────────────────────────────────

pub struct Vault;

impl Vault {
    /// No-op constructor. Returns immediately without loading any model.
    pub async fn open(
        _model_path: &std::path::PathBuf,
        _tokenizer_path: &std::path::PathBuf,
        _db_path: &str,
    ) -> osmozzz_core::Result<Self> {
        Ok(Self)
    }

    // ─── Embedder trait methods ──────────────────────────────────────────────

    pub async fn upsert(&self, _doc: &Document) -> osmozzz_core::Result<()> {
        Ok(())
    }

    pub async fn search(&self, _query: &str, _limit: usize) -> osmozzz_core::Result<Vec<SearchResult>> {
        Ok(vec![])
    }

    pub async fn exists(&self, _checksum: &str) -> osmozzz_core::Result<bool> {
        Ok(false)
    }

    pub async fn count(&self) -> osmozzz_core::Result<usize> {
        Ok(0)
    }

    // ─── Extended methods ────────────────────────────────────────────────────

    pub fn embed_raw(&self, _text: &str) -> osmozzz_core::Result<Vec<f32>> {
        Err(OsmozzError::Storage("ONNX embedding removed".to_string()))
    }

    pub async fn heal(&self) -> osmozzz_core::Result<()> {
        Ok(())
    }

    pub async fn health_check(&self) -> osmozzz_core::Result<()> {
        Ok(())
    }

    pub async fn compact(&self) -> osmozzz_core::Result<()> {
        Ok(())
    }

    pub async fn count_source(&self, _source: &str) -> osmozzz_core::Result<usize> {
        Ok(0)
    }

    pub async fn delete_by_source(&self, _source: &str) -> osmozzz_core::Result<()> {
        Ok(())
    }

    pub async fn store_text_only(&self, _doc: &Document) -> osmozzz_core::Result<()> {
        Ok(())
    }

    pub async fn get_full_content_by_url(&self, _url: &str) -> osmozzz_core::Result<Option<(Option<String>, String)>> {
        Ok(None)
    }

    pub async fn get_docs_info_by_urls(&self, _urls: &[String]) -> osmozzz_core::Result<Vec<(String, String, Option<String>, String)>> {
        Ok(vec![])
    }

    pub async fn get_emails_by_sender_and_date(&self, _pattern: &str, _from_ts: i64, _to_ts: i64, _limit: usize) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        Ok(vec![])
    }

    pub async fn get_emails_by_date(&self, _from_ts: i64, _to_ts: i64, _limit: usize) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        Ok(vec![])
    }

    pub async fn get_emails_by_sender(&self, _pattern: &str, _limit: usize) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        Ok(vec![])
    }

    pub async fn recent_emails_full(&self, _limit: usize) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        Ok(vec![])
    }

    pub async fn search_emails_by_keyword(&self, _keyword: &str, _limit: usize) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        Ok(vec![])
    }

    pub async fn search_by_keyword_source(
        &self,
        _keyword: &str,
        _limit: usize,
        _source: &str,
    ) -> osmozzz_core::Result<Vec<(Option<String>, String, String)>> {
        Ok(vec![])
    }

    pub async fn search_all_by_keyword(
        &self,
        _keyword: &str,
        _limit: usize,
    ) -> osmozzz_core::Result<Vec<(Option<String>, String, String, String)>> {
        Ok(vec![])
    }

    pub async fn search_grouped_by_keyword(
        &self,
        _keyword: &str,
        _per_source: usize,
    ) -> osmozzz_core::Result<HashMap<String, Vec<(i64, Option<String>, String, String)>>> {
        Ok(HashMap::new())
    }

    pub async fn search_by_keyword_dated(
        &self,
        _keyword: &str,
        _limit: usize,
        _source: &str,
    ) -> osmozzz_core::Result<Vec<(i64, Option<String>, String, String)>> {
        Ok(vec![])
    }

    pub async fn search_and_query(
        &self,
        _query: &str,
        _limit: usize,
    ) -> osmozzz_core::Result<Option<Vec<SearchResult>>> {
        Ok(None)
    }

    pub async fn search_filtered(
        &self,
        _query: &str,
        _limit: usize,
        _source_filter: Option<&str>,
    ) -> osmozzz_core::Result<Vec<SearchResult>> {
        Ok(vec![])
    }

    // ─── Blacklist methods ───────────────────────────────────────────────────

    pub fn load_blacklist(&self) -> Blacklist {
        Blacklist::load()
    }

    pub async fn ban_url(&self, url: &str) -> osmozzz_core::Result<()> {
        let mut bl = Blacklist::load();
        bl.ban_url(url);
        bl.save().map_err(|e| OsmozzError::Storage(e.to_string()))
    }

    pub async fn ban_source_item(&self, source: &str, identifier: &str) -> osmozzz_core::Result<()> {
        let mut bl = Blacklist::load();
        bl.ban_source_item(source, identifier);
        bl.save().map_err(|e| OsmozzError::Storage(e.to_string()))
    }

    pub async fn unban_url(&self, url: &str) -> osmozzz_core::Result<()> {
        let mut bl = Blacklist::load();
        bl.unban_url(url);
        bl.save().map_err(|e| OsmozzError::Storage(e.to_string()))
    }

    pub async fn unban_source_item(&self, source: &str, identifier: &str) -> osmozzz_core::Result<()> {
        let mut bl = Blacklist::load();
        bl.unban_source_item(source, identifier);
        bl.save().map_err(|e| OsmozzError::Storage(e.to_string()))
    }

    pub fn get_blacklist(&self) -> Blacklist {
        Blacklist::load()
    }

    // ─── iMessage ───────────────────────────────────────────────────────────

    pub async fn get_imessage_contacts(&self) -> osmozzz_core::Result<Vec<(String, String, i64, usize)>> {
        Ok(vec![])
    }

    pub async fn get_imessage_conversation(&self, _phone: &str, _limit: usize) -> osmozzz_core::Result<Vec<(i64, bool, String)>> {
        Ok(vec![])
    }

    pub async fn recent_emails(&self, _limit: usize) -> osmozzz_core::Result<Vec<SearchResult>> {
        Ok(vec![])
    }

    pub async fn recent_by_source(&self, _source: &str, _limit: usize) -> osmozzz_core::Result<Vec<SearchResult>> {
        Ok(vec![])
    }

    // ─── Metrics ────────────────────────────────────────────────────────────

    pub fn db_disk_bytes(&self) -> u64 {
        0
    }

    pub fn process_rss_mb() -> Option<u64> {
        let pid = std::process::id();
        let out = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        let kb: u64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
        Some(kb / 1024)
    }
}
