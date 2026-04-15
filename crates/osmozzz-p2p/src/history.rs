use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::io::Write;

/// Une entrée dans l'historique des requêtes reçues de peers.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryHistoryEntry {
    pub ts: i64,
    pub peer_id: String,
    pub peer_name: String,
    /// Pour "search" : la query texte. Pour "tool_call" : le nom du tool.
    pub query: String,
    pub results_count: usize,
    pub blocked: bool,   // true si la requête a été bloquée par les permissions
    /// "search" | "tool_call" — défaut "search" pour compatibilité ascendante
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Contenu brut du résultat (tronqué à 4 Ko) — None si bloqué ou vide
    #[serde(default)]
    pub data: Option<String>,
}

fn default_kind() -> String {
    "search".to_string()
}

pub struct QueryHistoryLog {
    path: PathBuf,
}

impl QueryHistoryLog {
    pub fn new() -> Result<Self> {
        let path = dirs_next::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Home introuvable"))?
            .join(".osmozzz/query_history.jsonl");
        Ok(Self { path })
    }

    /// Append une entrée (format JSONL — une ligne JSON par entrée)
    pub fn append(&self, entry: &QueryHistoryEntry) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(entry)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// Retourne les N dernières entrées (les plus récentes en premier)
    pub fn recent(&self, limit: usize) -> Vec<QueryHistoryEntry> {
        let content = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let mut entries: Vec<QueryHistoryEntry> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        entries.sort_by(|a, b| b.ts.cmp(&a.ts));
        entries.truncate(limit);
        entries
    }
}
