use crate::config::DatabaseConfig;
use aevum_common::{
    Error, Result,
    models::{DataPoint, Pipeline, Stream},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// Repository for stream metadata.
#[async_trait]
pub trait StreamRepository: Send + Sync {
    /// Get a stream by ID.
    async fn get(&self, id: &Uuid) -> Result<Option<Stream>>;

    /// List all streams.
    async fn list(&self) -> Result<Vec<Stream>>;
}

/// Repository for time-series data.
#[async_trait]
pub trait TimeSeriesRepository: Send + Sync {
    /// Query data points for a stream within a time range.
    async fn query(
        &self,
        stream_id: &Uuid,
        start_time: &DateTime<Utc>,
        end_time: &DateTime<Utc>,
        limit: Option<usize>,
    ) -> Result<Vec<DataPoint>>;

    /// Query aggregated data for a stream within a time range.
    async fn query_aggregated(
        &self,
        stream_id: &Uuid,
        start_time: &DateTime<Utc>,
        end_time: &DateTime<Utc>,
        interval: &str,
        function: &str,
        field: &str,
    ) -> Result<Vec<DataPoint>>;

    /// Query the latest data point for a stream.
    async fn query_latest(&self, stream_id: &Uuid) -> Result<Option<DataPoint>>;

    /// Get field statistics for a stream.
    async fn get_field_stats(
        &self,
        stream_id: &Uuid,
        field: &str,
        start_time: &DateTime<Utc>,
        end_time: &DateTime<Utc>,
    ) -> Result<FieldStats>;
}

/// Statistics for a field.
#[derive(Debug)]
pub struct FieldStats {
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

/// Postgres implementation of the stream repository.
pub struct PgStreamRepository {
    /// Database connection pool
    pool: PgPool,
}

#[async_trait]
impl StreamRepository for PgStreamRepository {
    async fn get(&self, id: &Uuid) -> Result<Option<Stream>> {
        debug!("Getting stream: {}", id);

        let record = sqlx::query!(
            r#"
            SELECT id, name, description, schema, retention_days, retention_max_records, created_at, updated_at
            FROM streams
            WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to get stream: {}", e)))?;

        match record {
            Some(record) => {
                let stream = Stream {
                    id: record.id,
                    name: record.name,
                    description: record.description,
                    schema: serde_json::from_value(record.schema)
                        .map_err(|e| Error::Serialization(e))?,
                    retention: aevum_common::models::RetentionConfig {
                        days: record.retention_days as u32,
                        max_records: record.retention_max_records.map(|v| v as u64),
                    },
                    created_at: record.created_at,
                    updated_at: record.updated_at,
                };

                Ok(Some(stream))
            }
            None => Ok(None),
        }
    }

    async fn list(&self) -> Result<Vec<Stream>> {
        debug!("Listing all streams");

        let records = sqlx::query!(
            r#"
            SELECT id, name, description, schema, retention_days, retention_max_records, created_at, updated_at
            FROM streams
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to list streams: {}", e)))?;

        let mut streams = Vec::with_capacity(records.len());

        for record in records {
            let stream = Stream {
                id: record.id,
                name: record.name,
                description: record.description,
                schema: serde_json::from_value(record.schema)
                    .map_err(|e| Error::Serialization(e))?,
                retention: aevum_common::models::RetentionConfig {
                    days: record.retention_days as u32,
                    max_records: record.retention_max_records.map(|v| v as u64),
                },
                created_at: record.created_at,
                updated_at: record.updated_at,
            };

            streams.push(stream);
        }

        Ok(streams)
    }
}

/// Postgres implementation of the time-series repository.
pub struct PgTimeSeriesRepository {
    /// Database connection pool
    pool: PgPool,
}

#[async_trait]
impl TimeSeriesRepository for PgTimeSeriesRepository {
    async fn query(
        &self,
        stream_id: &Uuid,
        start_time: &DateTime<Utc>,
        end_time: &DateTime<Utc>,
        limit: Option<usize>,
    ) -> Result<Vec<DataPoint>> {
        debug!(
            "Querying data points for stream: {} from {} to {}",
            stream_id, start_time, end_time
        );

        let limit_clause = limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default();

        let query = format!(
            r#"
            SELECT id, stream_id, timestamp, payload
            FROM data_points
            WHERE stream_id = $1 AND timestamp >= $2 AND timestamp <= $3
            ORDER BY timestamp
            {}
            "#,
            limit_clause
        );

        let rows = sqlx::query(&query)
            .bind(stream_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to query data points: {}", e)))?;

        let mut data_points = Vec::with_capacity(rows.len());

        for row in rows {
            let id: Uuid = row.get("id");
            let stream_id: Uuid = row.get("stream_id");
            let timestamp: DateTime<Utc> = row.get("timestamp");
            let payload: Value = row.get("payload");

            data_points.push(DataPoint {
                id,
                stream_id,
                timestamp,
                payload,
            });
        }

        Ok(data_points)
    }

    async fn query_aggregated(
        &self,
        stream_id: &Uuid,
        start_time: &DateTime<Utc>,
        end_time: &DateTime<Utc>,
        interval: &str,
        function: &str,
        field: &str,
    ) -> Result<Vec<DataPoint>> {
        debug!(
            "Querying aggregated data for stream: {} from {} to {} with interval {}",
            stream_id, start_time, end_time, interval
        );

        // Validate function
        let agg_function = match function.to_lowercase().as_str() {
            "sum" => "SUM",
            "avg" | "average" => "AVG",
            "min" => "MIN",
            "max" => "MAX",
            "count" => "COUNT",
            _ => {
                return Err(Error::Validation(format!(
                    "Invalid aggregation function: {}",
                    function
                )));
            }
        };

        // Build time bucket expression
        let time_bucket = format!("time_bucket('{}', timestamp)", interval);

        // Build field extraction expression
        let field_path = format!("payload->'{}'", field);

        let query = format!(
            r#"
            SELECT
                '{}' AS id,
                $1 AS stream_id,
                {time_bucket} AS timestamp,
                jsonb_build_object(
                    '{field}', {}({field_path})::text::numeric,
                    'count', COUNT(*)
                ) AS payload
            FROM data_points
            WHERE stream_id = $1 AND timestamp >= $2 AND timestamp <= $3
            GROUP BY {time_bucket}
            ORDER BY {time_bucket}
            "#,
            Uuid::new_v4(),
            agg_function
        );

        let rows = sqlx::query(&query)
            .bind(stream_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to query aggregated data: {}", e)))?;

        let mut data_points = Vec::with_capacity(rows.len());

        for row in rows {
            let id: Uuid = row.get("id");
            let stream_id: Uuid = row.get("stream_id");
            let timestamp: DateTime<Utc> = row.get("timestamp");
            let payload: Value = row.get("payload");

            data_points.push(DataPoint {
                id,
                stream_id,
                timestamp,
                payload,
            });
        }

        Ok(data_points)
    }

    async fn query_latest(&self, stream_id: &Uuid) -> Result<Option<DataPoint>> {
        debug!("Querying latest data point for stream: {}", stream_id);

        let row = sqlx::query(
            r#"
            SELECT id, stream_id, timestamp, payload
            FROM data_points
            WHERE stream_id = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(stream_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to query latest data point: {}", e)))?;

        match row {
            Some(row) => {
                let id: Uuid = row.get("id");
                let stream_id: Uuid = row.get("stream_id");
                let timestamp: DateTime<Utc> = row.get("timestamp");
                let payload: Value = row.get("payload");

                Ok(Some(DataPoint {
                    id,
                    stream_id,
                    timestamp,
                    payload,
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_field_stats(
        &self,
        stream_id: &Uuid,
        field: &str,
        start_time: &DateTime<Utc>,
        end_time: &DateTime<Utc>,
    ) -> Result<FieldStats> {
        debug!(
            "Getting field stats for stream: {} field: {} from {} to {}",
            stream_id, field, start_time, end_time
        );

        // Build field extraction expression
        let field_path = format!("payload->'{}'", field);

        let row = sqlx::query(&format!(
            r#"
            SELECT
                MIN(({field_path})::text::numeric) AS min,
                MAX(({field_path})::text::numeric) AS max,
                AVG(({field_path})::text::numeric) AS avg,
                STDDEV(({field_path})::text::numeric) AS stddev,
                COUNT(*) AS count,
                MIN(timestamp) AS first_timestamp,
                MAX(timestamp) AS last_timestamp
            FROM data_points
            WHERE 
                stream_id = $1 AND 
                timestamp >= $2 AND 
                timestamp <= $3 AND
                ({field_path}) IS NOT NULL
            "#
        ))
        .bind(stream_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to get field stats: {}", e)))?;

        Ok(FieldStats {
            min: row.get("min"),
            max: row.get("max"),
            avg: row.get("avg"),
            stddev: row.get("stddev"),
            count: row.get("count"),
            first_timestamp: row.get("first_timestamp"),
            last_timestamp: row.get("last_timestamp"),
        })
    }
}

/// Manager for all repositories.
#[derive(Clone)]
pub struct RepositoryManager {
    /// Stream repository
    stream_repo: Arc<dyn StreamRepository>,

    /// Time-series repository
    time_series_repo: Arc<dyn TimeSeriesRepository>,
}

impl RepositoryManager {
    /// Create a new repository manager.
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!("Initializing database connection");

        // Create connection pool
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(std::time::Duration::from_secs(
                config.connect_timeout_seconds,
            ))
            .connect(&config.url)
            .await
            .map_err(|e| Error::Database(format!("Failed to connect to database: {}", e)))?;

        // Create repositories
        let stream_repo =
            Arc::new(PgStreamRepository { pool: pool.clone() }) as Arc<dyn StreamRepository>;
        let time_series_repo = Arc::new(PgTimeSeriesRepository { pool: pool.clone() })
            as Arc<dyn TimeSeriesRepository>;

        Ok(Self {
            stream_repo,
            time_series_repo,
        })
    }

    /// Get the stream repository.
    pub fn stream_repository(&self) -> Arc<dyn StreamRepository> {
        Arc::clone(&self.stream_repo)
    }

    /// Get the time-series repository.
    pub fn time_series_repository(&self) -> Arc<dyn TimeSeriesRepository> {
        Arc::clone(&self.time_series_repo)
    }
}
