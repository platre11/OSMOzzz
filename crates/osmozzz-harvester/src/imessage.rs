use chrono::{TimeZone, Utc};
use osmozzz_core::{Document, OsmozzError, Result, SourceType};
use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tracing::{info, warn};

use crate::checksum;

/// Apple epoch offset: seconds between Unix epoch (1970) and Apple epoch (2001-01-01)
const APPLE_EPOCH_OFFSET: i64 = 978_307_200;

pub struct IMessageHarvester {
    db_path: PathBuf,
}

impl IMessageHarvester {
    pub fn new() -> Self {
        let db_path = dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join("Library/Messages/chat.db");
        Self { db_path }
    }

    fn shadow_copy(&self) -> Result<NamedTempFile> {
        if !self.db_path.exists() {
            return Err(OsmozzError::Harvester(format!(
                "iMessage DB not found at: {}",
                self.db_path.display()
            )));
        }
        let tmp = tempfile::NamedTempFile::new()
            .map_err(|e| OsmozzError::Harvester(format!("Temp file error: {}", e)))?;
        std::fs::copy(&self.db_path, tmp.path())
            .map_err(|e| OsmozzError::Harvester(format!("Copy failed: {}", e)))?;
        Ok(tmp)
    }

    /// Convert Apple timestamp to UTC.
    /// macOS High Sierra+: nanoseconds since 2001-01-01
    /// Older: seconds since 2001-01-01
    fn apple_ts_to_utc(ts: i64) -> Option<chrono::DateTime<Utc>> {
        let secs = if ts > 1_000_000_000_000 {
            ts / 1_000_000_000 + APPLE_EPOCH_OFFSET
        } else {
            ts + APPLE_EPOCH_OFFSET
        };
        Utc.timestamp_opt(secs, 0).single()
    }
}

impl Default for IMessageHarvester {
    fn default() -> Self {
        Self::new()
    }
}

impl osmozzz_core::Harvester for IMessageHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let shadow = match self.shadow_copy() {
            Ok(s) => s,
            Err(e) => {
                warn!("iMessage: {}", e);
                return Ok(vec![]);
            }
        };

        let conn = match Connection::open(shadow.path()) {
            Ok(c) => c,
            Err(e) => {
                warn!("iMessage SQLite open failed: {}", e);
                return Ok(vec![]);
            }
        };

        let mut stmt = match conn.prepare(
            r#"
            SELECT
                m.ROWID,
                m.text,
                m.date,
                m.is_from_me,
                h.id AS contact
            FROM message m
            LEFT JOIN handle h ON m.handle_id = h.ROWID
            WHERE m.text IS NOT NULL AND length(m.text) > 0
            ORDER BY m.date DESC
            LIMIT 5000
            "#,
        ) {
            Ok(s) => s,
            Err(e) => {
                warn!("iMessage prepare failed: {}", e);
                return Ok(vec![]);
            }
        };

        let mut documents = Vec::new();

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,            // ROWID
                    row.get::<_, String>(1)?,          // text
                    row.get::<_, i64>(2)?,             // date
                    row.get::<_, i32>(3)?,             // is_from_me
                    row.get::<_, Option<String>>(4)?,  // contact
                ))
            })
            .map_err(|e| OsmozzError::Harvester(format!("iMessage query failed: {}", e)))?;

        for row_result in rows {
            let (rowid, text, date, is_from_me, contact) = match row_result {
                Ok(r) => r,
                Err(_) => continue,
            };

            let contact_str = contact.as_deref().unwrap_or("inconnu");
            let direction = if is_from_me != 0 { "moi" } else { contact_str };
            let content = format!("[{}] {}", direction, text.trim());
            let checksum = checksum::compute(&content);

            let url = format!("imessage://msg/{}", rowid);
            let title = format!(
                "iMessage {} {}",
                if is_from_me != 0 { "→" } else { "←" },
                contact_str
            );

            let mut doc = Document::new(SourceType::IMessage, &url, &content, &checksum)
                .with_title(&title);

            if let Some(ts) = Self::apple_ts_to_utc(date) {
                doc = doc.with_source_ts(ts);
            }

            documents.push(doc);
        }

        info!("iMessage harvester found {} documents", documents.len());
        Ok(documents)
    }
}
