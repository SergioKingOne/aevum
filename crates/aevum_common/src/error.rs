use actix_web::{HttpResponse, error::ResponseError, http::StatusCode};
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

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Authentication(_) => StatusCode::UNAUTHORIZED,
            Self::Authorization(_) => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .json(serde_json::json!({ "error": self.to_string() }))
    }
}

/// Result type alias using the common Error type.
pub type Result<T> = std::result::Result<T, Error>;
