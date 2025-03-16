use crate::repository::{FieldStats, RepositoryManager};
use aevum_common::{Error, Result, models::DataPoint};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// Query parameters for data retrieval.
#[derive(Debug, Deserialize)]
pub struct DataQuery {
    /// Start time for the query (ISO 8601 format)
    pub start: Option<String>,

    /// End time for the query (ISO 8601 format)
    pub end: Option<String>,

    /// Maximum number of data points to return
    pub limit: Option<usize>,
}

/// Response for data query.
#[derive(Serialize)]
pub struct DataQueryResponse {
    /// Array of data points
    pub data: Vec<DataPoint>,

    /// Query metadata
    pub meta: QueryMetadata,
}

/// Query metadata.
#[derive(Serialize)]
pub struct QueryMetadata {
    /// Stream ID
    pub stream_id: Uuid,

    /// Start time of the query
    pub start_time: DateTime<Utc>,

    /// End time of the query
    pub end_time: DateTime<Utc>,

    /// Number of data points returned
    pub count: usize,
}

/// Query parameters for data aggregation.
#[derive(Debug, Deserialize)]
pub struct AggregationQuery {
    /// Start time for the query (ISO 8601 format)
    pub start: Option<String>,

    /// End time for the query (ISO 8601 format)
    pub end: Option<String>,

    /// Aggregation interval (e.g., '1 hour', '5 minutes')
    pub interval: String,

    /// Aggregation function (sum, avg, min, max, count)
    pub function: String,

    /// Field to aggregate
    pub field: String,
}

/// Response for field statistics.
#[derive(Serialize)]
pub struct FieldStatsResponse {
    /// Stream ID
    pub stream_id: Uuid,

    /// Field name
    pub field: String,

    /// Minimum value
    pub min: f64,

    /// Maximum value
    pub max: f64,

    /// Average value
    pub avg: f64,

    /// Standard deviation
    pub stddev: f64,

    /// Number of data points
    pub count: i64,

    /// First timestamp in the range
    pub first_timestamp: DateTime<Utc>,

    /// Last timestamp in the range
    pub last_timestamp: DateTime<Utc>,
}

/// Handler for querying data points.
pub async fn query_data(
    State(repos): State<Arc<RepositoryManager>>,
    Path(stream_id): Path<Uuid>,
    Query(query): Query<DataQuery>,
) -> Result<(StatusCode, Json<DataQueryResponse>)> {
    info!("Querying data for stream: {}", stream_id);

    // Check if stream exists
    let stream = repos
        .stream_repository()
        .get(&stream_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Stream not found: {}", stream_id)))?;

    // Parse time range
    let (start_time, end_time) = parse_time_range(&query.start, &query.end)?;

    // Query data
    let data = repos
        .time_series_repository()
        .query(&stream_id, &start_time, &end_time, query.limit)
        .await?;

    let metadata = QueryMetadata {
        stream_id,
        start_time,
        end_time,
        count: data.len(),
    };

    let response = DataQueryResponse {
        data,
        meta: metadata,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for aggregating data.
pub async fn aggregate_data(
    State(repos): State<Arc<RepositoryManager>>,
    Path(stream_id): Path<Uuid>,
    Query(query): Query<AggregationQuery>,
) -> Result<(StatusCode, Json<DataQueryResponse>)> {
    info!("Aggregating data for stream: {}", stream_id);

    // Check if stream exists
    let stream = repos
        .stream_repository()
        .get(&stream_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Stream not found: {}", stream_id)))?;

    // Parse time range
    let (start_time, end_time) = parse_time_range(&query.start, &query.end)?;

    // Query aggregated data
    let data = repos
        .time_series_repository()
        .query_aggregated(
            &stream_id,
            &start_time,
            &end_time,
            &query.interval,
            &query.function,
            &query.field,
        )
        .await?;

    let metadata = QueryMetadata {
        stream_id,
        start_time,
        end_time,
        count: data.len(),
    };

    let response = DataQueryResponse {
        data,
        meta: metadata,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for retrieving the latest data point.
pub async fn get_latest(
    State(repos): State<Arc<RepositoryManager>>,
    Path(stream_id): Path<Uuid>,
) -> Result<(StatusCode, Json<Option<DataPoint>>)> {
    info!("Getting latest data point for stream: {}", stream_id);

    // Check if stream exists
    let stream = repos
        .stream_repository()
        .get(&stream_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Stream not found: {}", stream_id)))?;

    // Query latest data point
    let data_point = repos
        .time_series_repository()
        .query_latest(&stream_id)
        .await?;

    Ok((StatusCode::OK, Json(data_point)))
}

/// Handler for getting field statistics.
pub async fn get_field_stats(
    State(repos): State<Arc<RepositoryManager>>,
    Path((stream_id, field)): Path<(Uuid, String)>,
    Query(query): Query<DataQuery>,
) -> Result<(StatusCode, Json<FieldStatsResponse>)> {
    info!(
        "Getting field stats for stream: {} field: {}",
        stream_id, field
    );

    // Check if stream exists
    let stream = repos
        .stream_repository()
        .get(&stream_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Stream not found: {}", stream_id)))?;

    // Parse time range
    let (start_time, end_time) = parse_time_range(&query.start, &query.end)?;

    // Get field stats
    let stats = repos
        .time_series_repository()
        .get_field_stats(&stream_id, &field, &start_time, &end_time)
        .await?;

    let response = FieldStatsResponse {
        stream_id,
        field,
        min: stats.min,
        max: stats.max,
        avg: stats.avg,
        stddev: stats.stddev,
        count: stats.count,
        first_timestamp: stats.first_timestamp,
        last_timestamp: stats.last_timestamp,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Parse start and end time from query parameters.
fn parse_time_range(
    start: &Option<String>,
    end: &Option<String>,
) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let end_time = match end {
        Some(end_str) => DateTime::parse_from_rfc3339(end_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::Validation(format!("Invalid end time format: {}", e)))?,
        None => Utc::now(),
    };

    let start_time = match start {
        Some(start_str) => DateTime::parse_from_rfc3339(start_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::Validation(format!("Invalid start time format: {}", e)))?,
        None => end_time - Duration::days(1), // Default to 1 day before end time
    };

    if start_time > end_time {
        return Err(Error::Validation(
            "Start time must be before end time".to_string(),
        ));
    }

    Ok((start_time, end_time))
}
