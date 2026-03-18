use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};
use osmozzz_core::{Document, OsmozzError, Result, SourceType};
use rusqlite::Connection;
use tempfile::NamedTempFile;
use tracing::{debug, info, warn};

use crate::checksum;

// Arc uses Chromium's WebKit timestamp format
const CHROME_EPOCH_OFFSET_MICROS: i64 = 11_644_473_600_000_000;

pub struct ArcHarvester {
    history_path: PathBuf,
}

impl ArcHarvester {
    pub fn new() -> Self {
        let history_path = dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join("Library/Application Support/Arc/User Data/Default/History");
        Self { history_path }
    }

    fn shadow_copy(&self) -> Result<NamedTempFile> {
        if !self.history_path.exists() {
            return Err(OsmozzError::Harvester(format!(
                "Arc history not found at: {}",
                self.history_path.display()
            )));
        }
        let tmp = tempfile::NamedTempFile::new()
            .map_err(|e| OsmozzError::Harvester(format!("Temp file error: {e}")))?;
        std::fs::copy(&self.history_path, tmp.path())
            .map_err(|e| OsmozzError::Harvester(format!("Copy failed: {e}")))?;
        Ok(tmp)
    }

    fn webkit_to_utc(ts: i64) -> Option<DateTime<Utc>> {
        if ts == 0 { return None; }
        let unix_micros = ts - CHROME_EPOCH_OFFSET_MICROS;
        let secs  = unix_micros / 1_000_000;
        let nanos = ((unix_micros % 1_000_000) * 1000) as u32;
        Utc.timestamp_opt(secs, nanos).single()
    }
}

impl Default for ArcHarvester {
    fn default() -> Self { Self::new() }
}

impl osmozzz_core::Harvester for ArcHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let shadow = match self.shadow_copy() {
            Ok(s) => s,
            Err(e) => { warn!("Arc: {e}"); return Ok(vec![]); }
        };

        let conn = match Connection::open(shadow.path()) {
            Ok(c) => c,
            Err(e) => { warn!("Arc SQLite open failed: {e}"); return Ok(vec![]); }
        };

        let mut stmt = match conn.prepare(r#"
            SELECT u.url, u.title, v.visit_time
            FROM urls u
            JOIN visits v ON v.url = u.id
            WHERE u.url NOT LIKE 'chrome://%'
              AND u.url NOT LIKE 'chrome-extension://%'
              AND u.url NOT LIKE 'about:%'
              AND length(u.title) > 0
            ORDER BY v.visit_time DESC
            LIMIT 10000
        "#) {
            Ok(s) => s,
            Err(e) => { warn!("Arc prepare failed: {e}"); return Ok(vec![]); }
        };

        let mut documents = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        }).map_err(|e| OsmozzError::Harvester(format!("Arc query failed: {e}")))?;

        for row_result in rows {
            let (url, title, visit_time) = match row_result {
                Ok(r) => r,
                Err(_) => continue,
            };
            if !seen.insert(url.clone()) { continue; }

            let content  = format!("{}\n{}", title.trim(), url.trim());
            let chk      = checksum::compute(&content);
            debug!("Arc URL: {}", url);

            let mut doc = Document::new(SourceType::Arc, &url, &content, &chk)
                .with_title(&title);
            if let Some(ts) = Self::webkit_to_utc(visit_time) {
                doc = doc.with_source_ts(ts);
            }
            documents.push(doc);
        }

        info!("Arc harvester found {} documents", documents.len());
        Ok(documents)
    }
}
