//! Typed error handling for music_fusion.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FusionError {
    #[error("Ollama service unreachable at {endpoint}: {source}")]
    OllamaUnreachable {
        endpoint: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Ollama API error ({status}): {message}")]
    OllamaApi {
        status: reqwest::StatusCode,
        message: String,
    },

    #[error("Model '{model}' not found in Ollama tags. Available: {available:?}")]
    ModelNotFound {
        model: String,
        available: Vec<String>,
    },

    #[error("HTTP client error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Hardware probe failure: {0}")]
    Hardware(String),

    #[error("Music producer panel error: {0}")]
    Panel(String),

    #[error("General error: {0}")]
    General(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, FusionError>;
