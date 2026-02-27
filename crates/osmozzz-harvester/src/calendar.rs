use chrono::{TimeZone, Utc};
use osmozzz_core::{Document, Result, SourceType};
use std::path::PathBuf;
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::checksum;

pub struct CalendarHarvester {
    calendars_dir: PathBuf,
}

impl CalendarHarvester {
    pub fn new() -> Self {
        let calendars_dir = dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join("Library/Calendars");
        Self { calendars_dir }
    }
}

impl Default for CalendarHarvester {
    fn default() -> Self {
        Self::new()
    }
}

impl osmozzz_core::Harvester for CalendarHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        if !self.calendars_dir.exists() {
            warn!("Calendar dir not found at: {}", self.calendars_dir.display());
            return Ok(vec![]);
        }

        let mut documents = Vec::new();

        // Walk all .ics files in ~/Library/Calendars/
        for entry in WalkDir::new(&self.calendars_dir)
            .max_depth(6)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "ics").unwrap_or(false))
        {
            let content = match std::fs::read_to_string(entry.path()) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Parse key fields from iCalendar format
            let summary = extract_ics_field(&content, "SUMMARY");
            let description = extract_ics_field(&content, "DESCRIPTION");
            let dtstart = extract_ics_field(&content, "DTSTART");
            let uid = extract_ics_field(&content, "UID");

            if summary.is_empty() && description.is_empty() {
                continue;
            }

            let text = if !description.is_empty() {
                format!("{}\n{}", summary, description)
            } else {
                summary.clone()
            };

            let checksum = checksum::compute(&text);
            let url = format!("calendar://event/{}", uid.replace('/', "_"));

            let source_ts = dtstart
                .chars()
                .take(8)
                .collect::<String>()
                .parse::<u32>()
                .ok()
                .and_then(|d| {
                    // DTSTART format: YYYYMMDD or YYYYMMDDTHHmmssZ
                    let year = d / 10000;
                    let month = (d % 10000) / 100;
                    let day = d % 100;
                    chrono::NaiveDate::from_ymd_opt(year as i32, month, day)
                        .and_then(|nd| nd.and_hms_opt(0, 0, 0))
                        .map(|ndt| Utc.from_utc_datetime(&ndt))
                });

            let mut doc = Document::new(SourceType::Calendar, &url, &text, &checksum);
            if !summary.is_empty() {
                doc = doc.with_title(&summary);
            }
            if let Some(ts) = source_ts {
                doc = doc.with_source_ts(ts);
            }

            documents.push(doc);
        }

        info!("Calendar harvester found {} events", documents.len());
        Ok(documents)
    }
}

/// Extract the value of a field from iCalendar text (handles line folding).
fn extract_ics_field(content: &str, field: &str) -> String {
    let prefix = format!("{}:", field);
    let prefix_param = format!("{};" , field); // DTSTART;TZID=...

    for line in content.lines() {
        if line.starts_with(&prefix) {
            return line[prefix.len()..].trim().to_string();
        }
        if line.starts_with(&prefix_param) {
            if let Some(colon) = line.find(':') {
                return line[colon + 1..].trim().to_string();
            }
        }
    }
    String::new()
}
