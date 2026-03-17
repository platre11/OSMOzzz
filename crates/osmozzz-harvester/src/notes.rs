use osmozzz_core::{Document, Result, SourceType};
use tracing::{info, warn};

use crate::checksum;

pub struct NotesHarvester;

impl NotesHarvester {
    pub fn new() -> Self { Self }
}

impl Default for NotesHarvester {
    fn default() -> Self { Self::new() }
}

impl osmozzz_core::Harvester for NotesHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        // AppleScript : récupère toutes les notes (id + nom + texte brut)
        let script = r#"tell application "Notes"
            set sep to "|||OSMOZZZ|||"
            set rec to "~~~OSMOZZZ~~~"
            set output to ""
            repeat with n in every note
                try
                    set nId to id of n
                    set nName to name of n
                    set nBody to plaintext of n
                    set output to output & nId & sep & nName & sep & nBody & rec
                end try
            end repeat
            return output
        end tell"#;

        let raw = match run_osascript(script).await {
            Ok(r) => r,
            Err(e) => {
                warn!("Notes AppleScript failed: {e}");
                return Ok(vec![]);
            }
        };

        let mut documents = Vec::new();

        for record in raw.split("~~~OSMOZZZ~~~") {
            let parts: Vec<&str> = record.splitn(3, "|||OSMOZZZ|||").collect();
            if parts.len() < 2 { continue; }

            let note_id = parts[0].trim();
            let title   = parts[1].trim();
            let body    = if parts.len() > 2 { parts[2].trim() } else { "" };

            if note_id.is_empty() || title.is_empty() { continue; }

            let content  = if body.is_empty() {
                title.to_string()
            } else {
                // Limite le body à 2000 chars pour éviter les notes gigantesques
                let body_short = body.chars().take(2000).collect::<String>();
                format!("{}\n{}", title, body_short)
            };

            let chk = checksum::compute(&content);
            let url = format!("notes://note/{}", note_id.replace(' ', "_"));

            let doc = Document::new(SourceType::Notes, &url, &content, &chk)
                .with_title(title);

            documents.push(doc);
        }

        info!("Notes harvester found {} documents", documents.len());
        Ok(documents)
    }
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
