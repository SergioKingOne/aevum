use crate::producer::KafkaProducer;
use actix_web::{
    HttpResponse,
    web::{Data, Json, Path},
};
use aevum_common::{
    Error, Result,
    models::DataPoint,
    utils::{current_timestamp, generate_id},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Request for single data point ingestion.
#[derive(Deserialize)]
pub struct IngestRequest {
    /// The data payload to ingest
    pub data: Value,

    /// Optional timestamp for the data point
    pub timestamp: Option<String>,
}

/// Request for batch data ingestion.
#[derive(Deserialize)]
pub struct BatchIngestRequest {
    /// Array of data payloads to ingest
    pub data: Vec<Value>,

    /// Optional timestamp for all data points (if not provided individually)
    pub timestamp: Option<String>,
}

/// Response for both single and batch ingestion.
#[derive(Serialize)]
pub struct IngestResponse {
    /// Number of data points successfully ingested
    pub ingested: usize,

    /// Array of IDs for the ingested data points
    pub ids: Vec<Uuid>,
}

/// Handler for ingesting a single data point.
pub async fn ingest_data(
    producer: Data<Arc<KafkaProducer>>,
    path: Path<(Uuid,)>,
    Json(request): Json<IngestRequest>,
) -> Result<HttpResponse> {
    let stream_id = path.into_inner().0;
    info!("Ingesting data point for stream: {}", stream_id);

    // TODO: We would fetch the stream from a repository
    // For MVP, we'll skip validation and assume the stream exists

    // Create data point
    let data_point = DataPoint {
        stream_id,
        id: generate_id(),
        timestamp: parse_timestamp(request.timestamp)?,
        payload: request.data,
    };

    // Send to Kafka
    producer.send_data_point(&data_point).await?;

    // Create response
    let response = IngestResponse {
        ingested: 1,
        ids: vec![data_point.id],
    };

    Ok(HttpResponse::Created().json(response))
}

/// Handler for ingesting a batch of data points.
pub async fn ingest_batch(
    producer: Data<Arc<KafkaProducer>>,
    path: Path<(Uuid,)>,
    Json(request): Json<BatchIngestRequest>,
) -> Result<HttpResponse> {
    let stream_id = path.into_inner().0;
    let count = request.data.len();
    info!(
        "Ingesting batch of {} data points for stream: {}",
        count, stream_id
    );

    if count == 0 {
        return Err(Error::Validation("Batch cannot be empty".to_string()));
    }

    // TODO: We would fetch the stream from a repository
    // For MVP, we'll skip validation and assume the stream exists

    // Create data points
    let default_timestamp = parse_timestamp(request.timestamp.clone())?;
    let mut data_points = Vec::with_capacity(count);
    let mut ids = Vec::with_capacity(count);

    for data in request.data {
        let id = generate_id();
        ids.push(id);

        data_points.push(DataPoint {
            stream_id,
            id,
            timestamp: default_timestamp,
            payload: data,
        });
    }

    // Send to Kafka
    producer.send_data_points(&data_points).await?;

    // Create response
    let response = IngestResponse {
        ingested: count,
        ids,
    };

    Ok(HttpResponse::Created().json(response))
}

/// Parse timestamp from string or use current time.
fn parse_timestamp(timestamp_str: Option<String>) -> Result<chrono::DateTime<chrono::Utc>> {
    match timestamp_str {
        Some(ts) => chrono::DateTime::parse_from_rfc3339(&ts)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .map_err(|e| Error::Validation(format!("Invalid timestamp format: {}", e))),
        None => Ok(current_timestamp()),
    }
}
