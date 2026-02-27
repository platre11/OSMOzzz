use chrono::{TimeZone, Utc};
use osmozzz_core::{Document, OsmozzError, Result, SourceType};
use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tracing::{info, warn};

use crate::checksum;

/// Apple CFAbsoluteTime: seconds since 2001-01-01
const APPLE_EPOCH_OFFSET: i64 = 978_307_200;

pub struct SafariHarvester {
    db_path: PathBuf,
}

impl SafariHarvester {
    pub fn new() -> Self {
        let db_path = dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join("Library/Safari/History.db");
        Self { db_path }
    }

    fn shadow_copy(&self) -> Result<NamedTempFile> {
        if !self.db_path.exists() {
            return Err(OsmozzError::Harvester(format!(
                "Safari History.db not found at: {}",
                self.db_path.display()
            )));
        }
        let tmp = tempfile::NamedTempFile::new()
            .map_err(|e| OsmozzError::Harvester(format!("Temp file error: {}", e)))?;
        std::fs::copy(&self.db_path, tmp.path())
            .map_err(|e| OsmozzError::Harvester(format!("Copy failed: {}", e)))?;
        Ok(tmp)
    }

    fn cf_time_to_utc(cf_time: f64) -> Option<chrono::DateTime<Utc>> {
        let secs = cf_time as i64 + APPLE_EPOCH_OFFSET;
        Utc.timestamp_opt(secs, 0).single()
    }
}

impl Default for SafariHarvester {
    fn default() -> Self {
        Self::new()
    }
}

impl osmozzz_core::Harvester for SafariHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let shadow = match self.shadow_copy() {
            Ok(s) => s,
            Err(e) => {
                warn!("Safari: {}", e);
                return Ok(vec![]);
            }
        };

        let conn = match Connection::open(shadow.path()) {
            Ok(c) => c,
            Err(e) => {
                warn!("Safari SQLite open failed: {}", e);
                return Ok(vec![]);
            }
        };

        let mut stmt = match conn.prepare(
            r#"
            SELECT
                hi.url,
                COALESCE(hv.title, hi.url) AS title,
                hv.visit_time
            FROM history_visits hv
            JOIN history_items hi ON hv.history_item = hi.id
            WHERE hi.url NOT LIKE 'safari://%'
                AND hi.url NOT LIKE 'about:%'
            ORDER BY hv.visit_time DESC
            LIMIT 10000
            "#,
        ) {
            Ok(s) => s,
            Err(e) => {
                warn!("Safari prepare failed: {}", e);
                return Ok(vec![]);
            }
        };

        let mut documents = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,  // url
                    row.get::<_, String>(1)?,  // title
                    row.get::<_, f64>(2)?,     // visit_time (CFAbsoluteTime)
                ))
            })
            .map_err(|e| OsmozzError::Harvester(format!("Safari query failed: {}", e)))?;

        for row_result in rows {
            let (url, title, visit_time) = match row_result {
                Ok(r) => r,
                Err(_) => continue,
            };

            if !seen_urls.insert(url.clone()) {
                continue;
            }

            let content = format!("{}\n{}", title.trim(), url.trim());
            let checksum = checksum::compute(&content);

            let mut doc = Document::new(SourceType::Safari, &url, &content, &checksum)
                .with_title(&title);

            if let Some(ts) = Self::cf_time_to_utc(visit_time) {
                doc = doc.with_source_ts(ts);
            }

            documents.push(doc);
        }

        info!("Safari harvester found {} documents", documents.len());
        Ok(documents)
    }
}
