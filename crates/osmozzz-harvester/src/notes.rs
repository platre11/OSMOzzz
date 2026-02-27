use chrono::{TimeZone, Utc};
use osmozzz_core::{Document, OsmozzError, Result, SourceType};
use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tracing::{info, warn};

use crate::checksum;

/// Apple CFAbsoluteTime: seconds since 2001-01-01
const APPLE_EPOCH_OFFSET: i64 = 978_307_200;

pub struct NotesHarvester {
    db_path: PathBuf,
}

impl NotesHarvester {
    pub fn new() -> Self {
        let db_path = dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join("Library/Group Containers/group.com.apple.notes/NoteStore.sqlite");
        Self { db_path }
    }

    fn shadow_copy(&self) -> Result<NamedTempFile> {
        if !self.db_path.exists() {
            return Err(OsmozzError::Harvester(format!(
                "NoteStore.sqlite not found at: {}",
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

impl Default for NotesHarvester {
    fn default() -> Self {
        Self::new()
    }
}

impl osmozzz_core::Harvester for NotesHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let shadow = match self.shadow_copy() {
            Ok(s) => s,
            Err(e) => {
                warn!("Notes: {}", e);
                return Ok(vec![]);
            }
        };

        let conn = match Connection::open(shadow.path()) {
            Ok(c) => c,
            Err(e) => {
                warn!("Notes SQLite open failed: {}", e);
                return Ok(vec![]);
            }
        };

        let mut stmt = match conn.prepare(
            r#"
            SELECT
                Z_PK,
                ZTITLE1,
                ZSNIPPET,
                ZMODIFICATIONDATE
            FROM ZICCLOUDSYNCINGOBJECT
            WHERE ZTITLE1 IS NOT NULL
                AND ZSNIPPET IS NOT NULL
                AND (ZMARKEDFORDELETION IS NULL OR ZMARKEDFORDELETION != 1)
            ORDER BY ZMODIFICATIONDATE DESC
            LIMIT 2000
            "#,
        ) {
            Ok(s) => s,
            Err(e) => {
                warn!("Notes prepare failed: {}", e);
                return Ok(vec![]);
            }
        };

        let mut documents = Vec::new();

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,   // Z_PK
                    row.get::<_, String>(1)?, // ZTITLE1
                    row.get::<_, String>(2)?, // ZSNIPPET
                    row.get::<_, f64>(3)?,    // ZMODIFICATIONDATE
                ))
            })
            .map_err(|e| OsmozzError::Harvester(format!("Notes query failed: {}", e)))?;

        for row_result in rows {
            let (pk, title, snippet, mod_date) = match row_result {
                Ok(r) => r,
                Err(_) => continue,
            };

            let content = format!("{}\n{}", title.trim(), snippet.trim());
            let checksum = checksum::compute(&content);
            let url = format!("notes://note/{}", pk);

            let mut doc = Document::new(SourceType::Notes, &url, &content, &checksum)
                .with_title(&title);

            if let Some(ts) = Self::cf_time_to_utc(mod_date) {
                doc = doc.with_source_ts(ts);
            }

            documents.push(doc);
        }

        info!("Notes harvester found {} documents", documents.len());
        Ok(documents)
    }
}
