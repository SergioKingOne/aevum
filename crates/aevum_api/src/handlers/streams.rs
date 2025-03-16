use crate::repository::RepositoryManager;
use aevum_common::{Error, Result, models::Stream};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// Response for listing streams.
#[derive(Serialize)]
pub struct ListStreamsResponse {
    /// Array of streams
    pub streams: Vec<Stream>,
}

/// Response for a single stream.
#[derive(Serialize)]
pub struct StreamResponse {
    /// The stream object
    pub stream: Stream,
}

/// Handler for listing all streams.
pub async fn list_streams(
    State(repos): State<Arc<RepositoryManager>>,
) -> Result<(StatusCode, Json<ListStreamsResponse>)> {
    info!("Listing all streams");

    let streams = repos.stream_repository().list().await?;

    let response = ListStreamsResponse { streams };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for getting a specific stream.
pub async fn get_stream(
    State(repos): State<Arc<RepositoryManager>>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<StreamResponse>)> {
    info!("Getting stream: {}", id);

    let stream = repos
        .stream_repository()
        .get(&id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Stream not found: {}", id)))?;

    let response = StreamResponse { stream };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for getting fields in a stream's schema.
pub async fn get_stream_fields(
    State(repos): State<Arc<RepositoryManager>>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<Vec<String>>)> {
    info!("Getting fields for stream: {}", id);

    let stream = repos
        .stream_repository()
        .get(&id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Stream not found: {}", id)))?;

    // Extract fields from the JSON schema
    let fields = extract_fields_from_schema(&stream.schema);

    Ok((StatusCode::OK, Json(fields)))
}

/// Extract field names from a JSON schema.
fn extract_fields_from_schema(schema: &serde_json::Value) -> Vec<String> {
    // This is a simplified implementation that expects a flat schema
    // In a real implementation, this would handle nested schemas, arrays, etc.

    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        return properties.keys().cloned().collect();
    }

    Vec::new()
}
