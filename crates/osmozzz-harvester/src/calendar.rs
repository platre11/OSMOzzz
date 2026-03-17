use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use osmozzz_core::{Document, Result, SourceType};
use tracing::{info, warn};

use crate::checksum;

pub struct CalendarHarvester;

impl CalendarHarvester {
    pub fn new() -> Self { Self }
}

impl Default for CalendarHarvester {
    fn default() -> Self { Self::new() }
}

impl osmozzz_core::Harvester for CalendarHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        // AppleScript : récupère les événements des 6 derniers mois + 1 an futur
        // Format date retourné : year§month§day§hour§minute
        let script = r#"tell application "Calendar"
            set sep to "|||OSMOZZZ|||"
            set rec to "~~~OSMOZZZ~~~"
            set output to ""
            set cutoff to (current date) - 180 * days
            set horizon to (current date) + 365 * days
            repeat with c in every calendar
                try
                    repeat with e in (every event of c whose start date >= cutoff and start date <= horizon)
                        try
                            set eTitle to summary of e
                            set sd to start date of e
                            set eDate to (year of sd as string) & "-" & (month of sd as integer as string) & "-" & (day of sd as string) & " " & (hours of sd as string) & ":" & (minutes of sd as string)
                            set eDesc to ""
                            try
                                if description of e is not missing value then
                                    set eDesc to description of e
                                end if
                            end try
                            set output to output & eTitle & sep & eDate & sep & eDesc & rec
                        end try
                    end repeat
                end try
            end repeat
            return output
        end tell"#;

        let raw = match run_osascript(script).await {
            Ok(r) => r,
            Err(e) => {
                warn!("Calendar AppleScript failed: {e}");
                return Ok(vec![]);
            }
        };

        let mut documents = Vec::new();

        for record in raw.split("~~~OSMOZZZ~~~") {
            let parts: Vec<&str> = record.splitn(3, "|||OSMOZZZ|||").collect();
            if parts.len() < 2 { continue; }

            let title = parts[0].trim();
            let date_str = parts[1].trim();
            let desc  = if parts.len() > 2 { parts[2].trim() } else { "" };

            if title.is_empty() { continue; }

            let content = if desc.is_empty() {
                format!("{}\n{}", title, date_str)
            } else {
                format!("{}\n{}\n{}", title, date_str, desc)
            };

            let chk = checksum::compute(&content);
            let url = format!("calendar://event/{}", checksum::compute(&format!("{}{}", title, date_str)));

            let source_ts = parse_date(date_str);

            let mut doc = Document::new(SourceType::Calendar, &url, &content, &chk)
                .with_title(title);
            if let Some(ts) = source_ts {
                doc = doc.with_source_ts(ts);
            }

            documents.push(doc);
        }

        info!("Calendar harvester found {} events", documents.len());
        Ok(documents)
    }
}

/// Parse "YYYY-M-D H:MM" from AppleScript output
fn parse_date(s: &str) -> Option<chrono::DateTime<Utc>> {
    // Format: "2026-3-17 14:0"
    let ndt = NaiveDateTime::parse_from_str(s, "%Y-%-m-%-d %-H:%-M").ok()
        .or_else(|| {
            // Fallback: try just date part
            let date_part = s.split_whitespace().next()?;
            NaiveDate::parse_from_str(date_part, "%Y-%-m-%-d").ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
        })?;
    Some(Utc.from_utc_datetime(&ndt))
}

async fn run_osascript(script: &str) -> std::result::Result<String, String> {
    let output = tokio::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .await
        .map_err(|e| format!("osascript spawn: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
