/// Obsidian Harvester — indexe les notes Markdown de votre vault Obsidian.
/// Aucune API nécessaire : lecture directe des fichiers .md locaux.
///
/// Config : ~/.osmozzz/obsidian.toml
/// ```toml
/// vault_path = "~/Documents/MyVault"
/// ```
///
/// Le vault_path est le dossier racine de votre vault Obsidian.
use osmozzz_core::{Document, Result, SourceType};
use std::path::PathBuf;
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::checksum;

const MAX_FILES: usize = 5000;

#[derive(Debug, serde::Deserialize)]
struct ObsidianConfig {
    vault_path: String,
}

impl ObsidianConfig {
    fn load() -> Option<PathBuf> {
        let path = dirs_next::home_dir()?.join(".osmozzz/obsidian.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let cfg: ObsidianConfig = toml::from_str(&content).ok()?;

        // Expand ~ manuellement
        let expanded = if cfg.vault_path.starts_with("~/") {
            dirs_next::home_dir()?
                .join(&cfg.vault_path[2..])
        } else {
            PathBuf::from(&cfg.vault_path)
        };
        Some(expanded)
    }
}

pub struct ObsidianHarvester;

impl ObsidianHarvester {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ObsidianHarvester {
    fn default() -> Self {
        Self::new()
    }
}

/// Supprime le frontmatter YAML d'un fichier Markdown
fn strip_frontmatter(content: &str) -> &str {
    if !content.starts_with("---") {
        return content;
    }
    // Trouver la fin du frontmatter
    let after_first = &content[3..];
    if let Some(pos) = after_first.find("\n---") {
        &content[3 + pos + 4..]
    } else {
        content
    }
}

impl osmozzz_core::Harvester for ObsidianHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let vault_path = match ObsidianConfig::load() {
            Some(p) => p,
            None => {
                warn!("Obsidian: ~/.osmozzz/obsidian.toml non trouvé — source ignorée");
                return Ok(vec![]);
            }
        };

        if !vault_path.exists() {
            warn!(
                "Obsidian: vault '{}' introuvable",
                vault_path.display()
            );
            return Ok(vec![]);
        }

        let mut documents = Vec::new();

        for entry in WalkDir::new(&vault_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file()
                    && e.path()
                        .extension()
                        .map(|ext| ext == "md")
                        .unwrap_or(false)
                    // Ignorer les dossiers .obsidian (config interne)
                    && !e.path().to_str().unwrap_or("").contains("/.obsidian/")
            })
        {
            if documents.len() >= MAX_FILES {
                warn!("Obsidian: limite de {} fichiers atteinte", MAX_FILES);
                break;
            }

            let path = entry.path();
            let content_raw = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let body = strip_frontmatter(&content_raw).trim().to_string();
            if body.is_empty() {
                continue;
            }

            // Titre = nom du fichier sans extension
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Sans titre");

            let content = format!("{}\n{}", title, body);
            let ck = checksum::compute(&content);

            let url = format!("obsidian://open?vault=vault&file={}", title);

            let mut doc =
                Document::new(SourceType::Obsidian, &url, &content, &ck)
                    .with_title(title);

            // Date de modification du fichier
            if let Ok(metadata) = path.metadata() {
                if let Ok(modified) = metadata.modified() {
                    let ts: chrono::DateTime<chrono::Utc> = modified.into();
                    doc = doc.with_source_ts(ts);
                }
            }

            documents.push(doc);
        }

        info!("Obsidian harvester found {} notes", documents.len());
        Ok(documents)
    }
}
