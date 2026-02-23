use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Chrome,
    File,
    Pdf,
    Markdown,
    Email,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Chrome => write!(f, "chrome"),
            SourceType::File => write!(f, "file"),
            SourceType::Pdf => write!(f, "pdf"),
            SourceType::Markdown => write!(f, "markdown"),
            SourceType::Email => write!(f, "email"),
        }
    }
}

impl std::str::FromStr for SourceType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chrome" => Ok(SourceType::Chrome),
            "file" => Ok(SourceType::File),
            "pdf" => Ok(SourceType::Pdf),
            "markdown" => Ok(SourceType::Markdown),
            "email" => Ok(SourceType::Email),
            other => Err(format!("Unknown source type: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub source: SourceType,
    /// URL ou chemin de fichier
    pub url: String,
    pub title: Option<String>,
    /// Contenu textuel du chunk
    pub content: String,
    /// SHA-256 du contenu pour éviter la réindexation
    pub checksum: String,
    pub harvested_at: DateTime<Utc>,
    pub source_ts: Option<DateTime<Utc>>,
    /// Index du chunk dans le document (0-based)
    pub chunk_index: Option<u32>,
    /// Nombre total de chunks pour ce document
    pub chunk_total: Option<u32>,
}

impl Document {
    pub fn new(
        source: SourceType,
        url: impl Into<String>,
        content: impl Into<String>,
        checksum: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            url: url.into(),
            title: None,
            content: content.into(),
            checksum: checksum.into(),
            harvested_at: Utc::now(),
            source_ts: None,
            chunk_index: None,
            chunk_total: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_source_ts(mut self, ts: DateTime<Utc>) -> Self {
        self.source_ts = Some(ts);
        self
    }

    pub fn with_chunk(mut self, index: u32, total: u32) -> Self {
        self.chunk_index = Some(index);
        self.chunk_total = Some(total);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    /// Similarité cosinus [0.0 - 1.0]
    pub score: f32,
    pub source: String,
    pub url: String,
    pub title: Option<String>,
    /// Extrait du contenu (300 chars max)
    pub content: String,
    /// Position du chunk dans le document
    pub chunk_index: Option<u32>,
    pub chunk_total: Option<u32>,
}
