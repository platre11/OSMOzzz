use chrono::{TimeZone, Utc};
use osmozzz_core::{Document, Result, SourceType};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use tracing::{info, warn};

use crate::checksum;

/// Retourne le chemin de l'historique terminal selon la plateforme.
fn terminal_history_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        // PowerShell history
        dirs_next::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Microsoft/Windows/PowerShell/PSReadLine/ConsoleHost_history.txt")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        // Préfère zsh, fallback bash
        let zsh = home.join(".zsh_history");
        if zsh.exists() {
            zsh
        } else {
            home.join(".bash_history")
        }
    }
}

pub struct TerminalHarvester {
    history_path: PathBuf,
}

impl TerminalHarvester {
    pub fn new() -> Self {
        let history_path = terminal_history_path();
        Self { history_path }
    }
}

impl Default for TerminalHarvester {
    fn default() -> Self {
        Self::new()
    }
}

impl osmozzz_core::Harvester for TerminalHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        if !self.history_path.exists() {
            warn!("zsh_history not found at: {}", self.history_path.display());
            return Ok(vec![]);
        }

        let file = match std::fs::File::open(&self.history_path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Cannot open zsh_history: {}", e);
                return Ok(vec![]);
            }
        };

        let reader = BufReader::new(file);
        let mut commands: Vec<(String, Option<i64>)> = Vec::new();

        for line in reader.lines() {
            // zsh_history can have invalid UTF-8 — skip bad lines
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            // zsh extended history format: `: timestamp:duration;command`
            if let Some(rest) = line.strip_prefix(": ") {
                if let Some(semi) = rest.find(';') {
                    let ts_part = &rest[..semi];
                    let cmd = rest[semi + 1..].trim().to_string();
                    if cmd.is_empty() {
                        continue;
                    }
                    let ts = ts_part
                        .split(':')
                        .next()
                        .and_then(|s| s.parse::<i64>().ok());
                    commands.push((cmd, ts));
                }
            } else {
                // Plain format (no timestamp)
                let cmd = line.trim().to_string();
                if !cmd.is_empty() {
                    commands.push((cmd, None));
                }
            }
        }

        // Remove consecutive duplicates
        commands.dedup_by(|a, b| a.0 == b.0);

        // Keep last 5000 commands
        let start = commands.len().saturating_sub(5000);
        let commands = &commands[start..];

        let total = commands.len() as u32;
        let mut documents = Vec::new();

        for (i, (cmd, ts)) in commands.iter().enumerate() {
            let checksum = checksum::compute(cmd);
            let url = format!("terminal://history/{}", checksum);

            let source_ts = ts.and_then(|t| Utc.timestamp_opt(t, 0).single());

            let mut doc = Document::new(SourceType::Terminal, &url, cmd, &checksum)
                .with_chunk(i as u32, total);

            if let Some(ts) = source_ts {
                doc = doc.with_source_ts(ts);
            }

            documents.push(doc);
        }

        info!("Terminal harvester found {} commands", documents.len());
        Ok(documents)
    }
}
