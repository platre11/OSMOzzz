use thiserror::Error;

#[derive(Error, Debug)]
pub enum OsmozzError {
    #[error("Harvester error: {0}")]
    Harvester(String),

    #[error("Embedder error: {0}")]
    Embedder(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Bridge error: {0}")]
    Bridge(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Model not found at path: {0}")]
    ModelNotFound(String),

    #[error("Not initialized: {0}")]
    NotInitialized(String),
}

pub type Result<T> = std::result::Result<T, OsmozzError>;
