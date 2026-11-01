use thiserror::Error;

#[derive(Error, Debug)]
pub enum MusicCoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("State validation error: {0}")]
    ValidationError(String),

    #[error("Genre not found in taxonomy: {0}")]
    GenreNotFound(String),

    #[error("Missing required stem: {0}")]
    MissingStem(String),

    #[error("Invalid BPM {0}: must be between 30 and 300")]
    InvalidBpm(u32),
}

pub type Result<T> = std::result::Result<T, MusicCoreError>;
