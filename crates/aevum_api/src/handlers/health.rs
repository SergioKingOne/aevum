use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};

/// Response for the health check endpoint.
#[derive(Serialize, Deserialize)]
pub struct HealthResponse {
    /// Status of the service
    pub status: String,

    /// Version of the service
    pub version: String,
}

/// Handler for the health check endpoint.
pub async fn check() -> (StatusCode, Json<HealthResponse>) {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    (StatusCode::OK, Json(response))
}
