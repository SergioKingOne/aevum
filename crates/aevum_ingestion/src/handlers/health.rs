use actix_web::HttpResponse;
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
pub async fn check() -> HttpResponse {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    HttpResponse::Ok().json(response)
}
