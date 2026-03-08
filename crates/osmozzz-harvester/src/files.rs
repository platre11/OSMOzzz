use std::path::{Path, PathBuf};

use chrono::DateTime;
use osmozzz_core::{Document, OsmozzError, Result, SourceType};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

use crate::checksum;
use crate::splitter::split_text;

// ~400 tokens pour all-MiniLM-L6-v2 (limite 512 tokens)
const MAX_CHARS: usize = 1600;
const OVERLAP_CHARS: usize = 160;
// Au-delà de 2 MB → on indexe seulement les métadonnées (pas le contenu)
const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;
// Fichiers binaires : pas de limite de taille (on stocke juste le nom/chemin)
const MAX_BINARY_SIZE: u64 = 500 * 1024 * 1024;
// PDFs > 5 Mo → métadonnées seulement (pdf_extract est trop gourmand en RAM)
pub const MAX_PDF_BYTES: u64 = 5 * 1024 * 1024;
// Dans le watcher : PDFs > 10 Mo → métadonnées seulement (évite les explosions RAM)
pub const MAX_PDF_WATCHER_BYTES: u64 = 10 * 1024 * 1024;

/// Dossiers à ignorer systématiquement (bruit, dépendances, caches)
pub const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "__pycache__",
    ".cargo",
    "dist",
    "build",
    ".next",
    ".nuxt",
    "vendor",
    ".build",
    "Pods",
    "DerivedData",
    ".gradle",
    ".idea",
    "venv",
    ".venv",
    "env",
    ".tox",
    ".osmozzz", // données internes → jamais indexées
];

/// Extensions connues comme texte lisible
pub const TEXT_EXTENSIONS: &[&str] = &[
    // Langages de programmation
    "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "h",
    "hpp", "cs", "swift", "kt", "rb", "php", "lua", "r", "scala", "dart",
    "vue", "svelte", "elm", "clj", "hs", "ex", "exs", "nim", "zig", "odin",
    "jai", "v", "d", "f90", "f95", "f03", "m", "mm",
    // Config & données
    "json", "yaml", "yml", "toml", "xml", "csv", "sql", "graphql", "proto",
    "ini", "cfg", "conf", "config", "env",
    // Docs & texte
    "md", "mdx", "txt", "log", "rst", "org", "tex", "adoc",
    // Shell
    "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd",
    // Web
    "html", "htm", "css", "scss", "sass", "less",
    // Divers
    "dockerfile", "makefile", "gitignore", "gitattributes", "editorconfig",
    "lock", "sum",
];

#[derive(Debug, Clone, Copy)]
enum FileKind {
    /// Texte lisible UTF-8 → contenu extrait + chunking
    Text,
    /// PDF → extraction via pdf_extract
    Pdf,
    /// Tout le reste → métadonnées uniquement (nom, chemin, extension)
    Binary,
}

pub struct FileHarvester {
    root_path: PathBuf,
    known_checksums: std::collections::HashSet<String>,
}

impl FileHarvester {
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
            known_checksums: Default::default(),
        }
    }

    pub fn with_known_checksums(
        self,
        checksums: std::collections::HashSet<String>,
    ) -> Self {
        Self { known_checksums: checksums, ..self }
    }
}

impl osmozzz_core::Harvester for FileHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        if !self.root_path.exists() {
            return Err(OsmozzError::Harvester(format!(
                "Path does not exist: {}",
                self.root_path.display()
            )));
        }

        let mut documents = Vec::new();

        for entry in WalkDir::new(&self.root_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_skipped(e.path()))
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Ignorer les fichiers cachés (.DS_Store, .swp, etc.)
            if is_hidden(path) {
                continue;
            }

            let meta = match std::fs::metadata(path) {
                Ok(m) => m,
                Err(e) => {
                    warn!("Metadata error {}: {}", path.display(), e);
                    continue;
                }
            };

            let mut docs = harvest_file(path, meta.len(), &self.known_checksums);
            documents.append(&mut docs);
        }

        info!(
            "FileHarvester: {} documents from {}",
            documents.len(),
            self.root_path.display()
        );
        Ok(documents)
    }
}

/// Indexe un fichier par ses métadonnées uniquement (nom, chemin, extension).
/// Utilisé par le watcher — aucune lecture du contenu, RAM stable.
pub fn harvest_file_metadata(path: &Path) -> Vec<Document> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("fichier")
        .to_string();
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    let content = format!(
        "Fichier: {}\nExtension: .{}\nChemin: {}",
        file_name, extension, path.display()
    );
    let ck = checksum::compute(&content);
    let url = format!("file://{}", path.display());
    let doc = Document::new(SourceType::File, &url, &content, &ck)
        .with_title(&file_name);
    vec![doc]
}

/// Indexe un dossier par son nom et son chemin (métadonnées uniquement).
pub fn harvest_directory(path: &Path) -> Vec<Document> {
    let dir_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("dossier")
        .to_string();

    let content = format!(
        "Dossier: {}\nChemin: {}",
        dir_name,
        path.display()
    );
    let ck = checksum::compute(&content);
    let url = format!("file://{}", path.display());

    let doc = Document::new(SourceType::File, &url, &content, &ck)
        .with_title(&dir_name);
    vec![doc]
}

/// Traite un seul fichier et retourne les Documents correspondants.
/// Utilisé à la fois par FileHarvester (scan initial) et par le Watcher (événements).
pub fn harvest_file(
    path: &Path,
    file_size: u64,
    known_checksums: &std::collections::HashSet<String>,
) -> Vec<Document> {
    let kind = detect_kind(path, file_size);

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&file_name)
        .to_string();

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();

    let base_url = format!("file://{}", path.display());

    let modified_ts = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<chrono::Utc>::from);

    match kind {
        FileKind::Binary => {
            // Indexe uniquement les métadonnées pour permettre la recherche par nom/chemin
            let content = format!(
                "File: {}\nExtension: .{}\nPath: {}",
                file_name,
                extension,
                path.display()
            );
            let ck = checksum::compute(&content);
            if known_checksums.contains(&ck) {
                debug!("Skipping known binary: {}", path.display());
                return vec![];
            }
            let mut doc = Document::new(SourceType::File, &base_url, &content, &ck)
                .with_title(&file_name);
            if let Some(ts) = modified_ts {
                doc = doc.with_source_ts(ts);
            }
            vec![doc]
        }

        FileKind::Text => {
            // Fichier trop grand → métadonnées seulement
            if file_size > MAX_TEXT_BYTES {
                debug!("Text file too large ({}), indexing metadata only: {}", file_size, path.display());
                let content = format!(
                    "File: {}\nExtension: .{}\nPath: {}\n(large file: {} KB)",
                    file_name,
                    extension,
                    path.display(),
                    file_size / 1024
                );
                let ck = checksum::compute(&content);
                if known_checksums.contains(&ck) {
                    return vec![];
                }
                let mut doc = Document::new(SourceType::File, &base_url, &content, &ck)
                    .with_title(&file_name);
                if let Some(ts) = modified_ts {
                    doc = doc.with_source_ts(ts);
                }
                return vec![doc];
            }

            let raw = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Read failed for {}: {}", path.display(), e);
                    return vec![];
                }
            };

            let text = raw.trim().to_string();
            if text.is_empty() {
                return vec![];
            }

            let chunks = split_text(&text, MAX_CHARS, OVERLAP_CHARS);
            let total = chunks.len() as u32;
            let mut docs = Vec::new();

            for (idx, chunk_text) in chunks.into_iter().enumerate() {
                let ck = checksum::compute(&chunk_text);
                if known_checksums.contains(&ck) {
                    debug!("Skipping known chunk: {} #{}", path.display(), idx);
                    continue;
                }

                let url = if total > 1 {
                    format!("{}#chunk-{}-of-{}", base_url, idx + 1, total)
                } else {
                    base_url.clone()
                };

                let title = if total > 1 {
                    format!("{} ({}/{})", file_stem, idx + 1, total)
                } else {
                    file_stem.clone()
                };

                let mut doc = Document::new(SourceType::File, &url, &chunk_text, &ck)
                    .with_title(&title)
                    .with_chunk(idx as u32, total);

                if let Some(ts) = modified_ts {
                    doc = doc.with_source_ts(ts);
                }
                docs.push(doc);
            }
            docs
        }

        FileKind::Pdf => {
            if file_size > MAX_PDF_BYTES {
                debug!("PDF too large ({}KB > 5MB), indexing metadata only: {}", file_size / 1024, path.display());
                // Métadonnées seulement pour les gros PDFs
                let content = format!(
                    "File: {}\nExtension: .pdf\nPath: {}\n(large PDF: {} KB)",
                    file_name, path.display(), file_size / 1024
                );
                let ck = checksum::compute(&content);
                if known_checksums.contains(&ck) { return vec![]; }
                let mut doc = Document::new(SourceType::File, &base_url, &content, &ck)
                    .with_title(&file_name);
                if let Some(ts) = modified_ts { doc = doc.with_source_ts(ts); }
                return vec![doc];
            }

            let full_text = match pdf_extract::extract_text(path) {
                Ok(t) => t,
                Err(e) => {
                    warn!("PDF extract failed for {}: {}", path.display(), e);
                    return vec![];
                }
            };

            let text = full_text.trim().to_string();
            if text.is_empty() {
                return vec![];
            }

            let chunks = split_text(&text, MAX_CHARS, OVERLAP_CHARS);
            let total = chunks.len() as u32;
            let mut docs = Vec::new();

            for (idx, chunk_text) in chunks.into_iter().enumerate() {
                let ck = checksum::compute(&chunk_text);
                if known_checksums.contains(&ck) {
                    continue;
                }

                let url = if total > 1 {
                    format!("{}#chunk-{}-of-{}", base_url, idx + 1, total)
                } else {
                    base_url.clone()
                };

                let title = if total > 1 {
                    format!("{} ({}/{})", file_stem, idx + 1, total)
                } else {
                    file_stem.clone()
                };

                let mut doc = Document::new(SourceType::Pdf, &url, &chunk_text, &ck)
                    .with_title(&title)
                    .with_chunk(idx as u32, total);

                if let Some(ts) = modified_ts {
                    doc = doc.with_source_ts(ts);
                }
                docs.push(doc);
            }
            docs
        }
    }
}

// ─── Détection du type de fichier ────────────────────────────────────────────

fn detect_kind(path: &Path, _file_size: u64) -> FileKind {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    // PDF → extraction dédiée
    if ext == "pdf" {
        return FileKind::Pdf;
    }

    // Extension connue comme texte
    if TEXT_EXTENSIONS.contains(&ext.as_str()) {
        return FileKind::Text;
    }

    // Extension inconnue → binaire (métadonnées seulement).
    // On n'essaie plus de lire le fichier pour détecter l'UTF-8 :
    // cela allouait jusqu'à 2 MB par fichier à chaque événement FSEvents.
    FileKind::Binary
}

// ─── Filtres ──────────────────────────────────────────────────────────────────


fn is_skipped(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|name| SKIP_DIRS.contains(&name))
        .unwrap_or(false)
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_pdf() {
        assert!(matches!(
            detect_kind(Path::new("doc.pdf"), 1000),
            FileKind::Pdf
        ));
    }

    #[test]
    fn test_detect_rust_code() {
        assert!(matches!(
            detect_kind(Path::new("main.rs"), 1000),
            FileKind::Text
        ));
    }

    #[test]
    fn test_detect_binary_gltf() {
        // .gltf n'est pas dans TEXT_EXTENSIONS, et on simule un fichier trop grand
        // pour le fallback UTF-8
        // Note: en vrai, .gltf est du JSON donc passerait en Text si petit
        assert!(matches!(
            detect_kind(Path::new("model.blend"), MAX_TEXT_BYTES + 1),
            FileKind::Binary
        ));
    }

    #[test]
    fn test_skip_node_modules() {
        assert!(is_skipped(Path::new("node_modules")));
        assert!(is_skipped(Path::new("target")));
        assert!(!is_skipped(Path::new("src")));
    }

    #[test]
    fn test_hidden_files() {
        assert!(is_hidden(Path::new(".DS_Store")));
        assert!(!is_hidden(Path::new("main.rs")));
    }
}
