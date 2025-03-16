use crate::{producer::KafkaProducer, validation::StreamValidator};
use actix_web::{
    HttpResponse,
    web::{Data, Json, Path},
};
use aevum_common::{Error, Result, models::Stream, utils::generate_id};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Request for creating a new stream.
#[derive(Deserialize)]
pub struct CreateStreamRequest {
    /// Human-readable name for the stream
    pub name: String,

    /// Optional description of the stream
    pub description: Option<String>,

    /// Schema definition for data in this stream (JSON Schema format)
    pub schema: serde_json::Value,

    /// Optional configuration for data retention
    pub retention: Option<RetentionRequest>,
}

/// Request for updating an existing stream.
#[derive(Deserialize)]
pub struct UpdateStreamRequest {
    /// Optional new name for the stream
    pub name: Option<String>,

    /// Optional new description for the stream
    pub description: Option<String>,

    /// Optional new schema for the stream
    pub schema: Option<serde_json::Value>,

    /// Optional new retention configuration
    pub retention: Option<RetentionRequest>,
}

/// Retention configuration in requests.
#[derive(Deserialize)]
pub struct RetentionRequest {
    /// How long to keep data in days
    pub days: u32,

    /// Maximum number of records to keep
    pub max_records: Option<u64>,
}

/// Response for stream operations.
#[derive(Serialize)]
pub struct StreamResponse {
    /// The stream object
    pub stream: Stream,
}

/// Response for listing streams.
#[derive(Serialize)]
pub struct ListStreamsResponse {
    /// Array of streams
    pub streams: Vec<Stream>,
}

/// Handler for creating a new stream.
pub async fn create_stream(
    producer: Data<Arc<KafkaProducer>>,
    Json(request): Json<CreateStreamRequest>,
) -> Result<HttpResponse> {
    info!("Creating new stream: {}", request.name);

    // Build stream from request
    let stream_id = generate_id();
    let mut stream = Stream::builder()
        .id(stream_id)
        .name(request.name)
        .schema(request.schema)
        .build();

    if let Some(description) = request.description {
        stream.description = Some(description);
    }

    if let Some(retention) = request.retention {
        stream.retention = aevum_common::models::RetentionConfig {
            days: retention.days,
            max_records: retention.max_records,
        };
    }

    // Validate stream
    StreamValidator::validate(&stream)?;

    // Send stream metadata to Kafka
    producer.send_stream_metadata(&stream.id, &stream).await?;

    // Return response
    let response = StreamResponse { stream };
    Ok(HttpResponse::Created().json(response))
}

/// Handler for listing all streams.
pub async fn list_streams() -> Result<HttpResponse> {
    info!("Listing all streams");

    // TODO: In a real implementation, we would fetch streams from a repository
    // For MVP, return an empty list
    let response = ListStreamsResponse { streams: vec![] };

    Ok(HttpResponse::Ok().json(response))
}

/// Handler for getting a specific stream.
pub async fn get_stream(path: Path<(Uuid,)>) -> Result<HttpResponse> {
    let id = path.into_inner().0;
    info!("Getting stream: {}", id);

    // TODO: We would fetch the stream from a repository
    // For MVP, return a not found error
    Err(Error::NotFound(format!("Stream not found: {}", id)))
}
