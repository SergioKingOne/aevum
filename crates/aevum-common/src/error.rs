use thiserror::Error;

/// Common error type for Aevum operations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Stream processing error: {0}")]
    StreamProcessing(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

/// Result type alias using the common Error type.
pub type Result<T> = std::result::Result<T, Error>;
