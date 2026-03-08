use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};
use osmozzz_core::{Document, OsmozzError, Result, SourceType};
use rusqlite::Connection;
use tempfile::NamedTempFile;
use tracing::{debug, info};

use crate::checksum;

/// Retourne le chemin de l'historique Chrome selon la plateforme.
fn chrome_history_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join("Library/Application Support/Google/Chrome/Default/History")
    }
    #[cfg(target_os = "windows")]
    {
        dirs_next::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Google/Chrome/User Data/Default/History")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".config/google-chrome/Default/History")
    }
}

/// Chrome WebKit epoch: microseconds since 1601-01-01
const CHROME_EPOCH_OFFSET_MICROS: i64 = 11_644_473_600_000_000;

/// Harvests browsing history from Google Chrome's SQLite database.
///
/// Uses a shadow copy to avoid conflicts with Chrome's write lock.
pub struct ChromeHarvester {
    /// Path to Chrome's History SQLite file
    history_path: PathBuf,
    /// Known checksums to skip (already indexed)
    known_checksums: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

impl ChromeHarvester {
    /// Create with default Chrome history path (cross-platform).
    pub fn new() -> Self {
        let history_path = chrome_history_path();
        Self {
            history_path,
            known_checksums: Default::default(),
        }
    }

    /// Create with a custom history path (useful for testing).
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            history_path: path.into(),
            known_checksums: Default::default(),
        }
    }

    /// Mark checksums as already known (skip during harvest).
    pub fn with_known_checksums(
        self,
        checksums: std::collections::HashSet<String>,
    ) -> Self {
        *self.known_checksums.lock().unwrap() = checksums;
        self
    }

    /// Creates a temporary shadow copy of the Chrome history DB.
    fn shadow_copy(&self) -> Result<NamedTempFile> {
        if !self.history_path.exists() {
            return Err(OsmozzError::Harvester(format!(
                "Chrome history not found at: {}",
                self.history_path.display()
            )));
        }
        let tmp = tempfile::NamedTempFile::new()
            .map_err(|e| OsmozzError::Harvester(format!("Failed to create temp file: {}", e)))?;
        std::fs::copy(&self.history_path, tmp.path())
            .map_err(|e| OsmozzError::Harvester(format!("Failed to copy history DB: {}", e)))?;
        Ok(tmp)
    }

    /// Convert Chrome's WebKit timestamp to UTC DateTime.
    fn webkit_to_utc(webkit_ts: i64) -> Option<DateTime<Utc>> {
        if webkit_ts == 0 {
            return None;
        }
        let unix_micros = webkit_ts - CHROME_EPOCH_OFFSET_MICROS;
        let secs = unix_micros / 1_000_000;
        let nanos = ((unix_micros % 1_000_000) * 1000) as u32;
        Utc.timestamp_opt(secs, nanos).single()
    }
}

impl Default for ChromeHarvester {
    fn default() -> Self {
        Self::new()
    }
}

impl osmozzz_core::Harvester for ChromeHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let shadow = self.shadow_copy()?;
        info!(
            "Shadow copy of Chrome history created at: {}",
            shadow.path().display()
        );

        let conn = Connection::open(shadow.path())
            .map_err(|e| OsmozzError::Harvester(format!("SQLite open failed: {}", e)))?;

        // Query visits joined with urls for title + timestamp
        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    u.url,
                    u.title,
                    u.visit_count,
                    v.visit_time
                FROM urls u
                JOIN visits v ON v.url = u.id
                WHERE
                    u.url NOT LIKE 'chrome://%'
                    AND u.url NOT LIKE 'chrome-extension://%'
                    AND u.url NOT LIKE 'about:%'
                    AND length(u.title) > 0
                ORDER BY v.visit_time DESC
                LIMIT 10000
                "#,
            )
            .map_err(|e| OsmozzError::Harvester(format!("SQLite prepare failed: {}", e)))?;

        let known = self.known_checksums.lock().unwrap().clone();
        let mut documents = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|e| OsmozzError::Harvester(format!("SQLite query failed: {}", e)))?;

        for row_result in rows {
            let (url, title, _visit_count, visit_time) =
                row_result.map_err(|e| OsmozzError::Harvester(format!("Row error: {}", e)))?;

            // Deduplicate by URL within this harvest batch
            if !seen_urls.insert(url.clone()) {
                continue;
            }

            // Content = title + url for embedding
            let content = format!("{}\n{}", title.trim(), url.trim());
            let checksum = checksum::compute(&content);

            if known.contains(&checksum) {
                debug!("Skipping already-indexed URL: {}", url);
                continue;
            }

            let mut doc = Document::new(SourceType::Chrome, &url, &content, &checksum)
                .with_title(&title);

            if let Some(ts) = Self::webkit_to_utc(visit_time) {
                doc = doc.with_source_ts(ts);
            }

            documents.push(doc);
        }

        info!("Chrome harvester found {} new documents", documents.len());
        Ok(documents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webkit_to_utc() {
        // 13_000_000_000_000_000 microseconds after Chrome epoch
        let ts = 13_000_000_000_000_000i64;
        let dt = ChromeHarvester::webkit_to_utc(ts);
        assert!(dt.is_some());
    }

    #[test]
    fn test_webkit_zero_returns_none() {
        assert!(ChromeHarvester::webkit_to_utc(0).is_none());
    }
}
