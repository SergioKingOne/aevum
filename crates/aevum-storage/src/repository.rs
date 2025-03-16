use crate::config::DatabaseConfig;
use aevum_common::{
    Error, Result,
    models::{DataPoint, Pipeline, Stream},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// Repository for stream metadata.
#[async_trait]
pub trait StreamRepository: Send + Sync {
    /// Create a new stream.
    async fn create(&self, stream: &Stream) -> Result<()>;

    /// Get a stream by ID.
    async fn get(&self, id: &Uuid) -> Result<Option<Stream>>;

    /// List all streams.
    async fn list(&self) -> Result<Vec<Stream>>;

    /// Update a stream.
    async fn update(&self, stream: &Stream) -> Result<()>;

    /// Delete a stream.
    async fn delete(&self, id: &Uuid) -> Result<()>;
}

/// Repository for pipeline configurations.
#[async_trait]
pub trait PipelineRepository: Send + Sync {
    /// Create a new pipeline.
    async fn create(&self, pipeline: &Pipeline) -> Result<()>;

    /// Get a pipeline by ID.
    async fn get(&self, id: &Uuid) -> Result<Option<Pipeline>>;

    /// List all pipelines.
    async fn list(&self) -> Result<Vec<Pipeline>>;

    /// List pipelines for a specific stream.
    async fn list_for_stream(&self, stream_id: &Uuid) -> Result<Vec<Pipeline>>;

    /// Update a pipeline.
    async fn update(&self, pipeline: &Pipeline) -> Result<()>;

    /// Delete a pipeline.
    async fn delete(&self, id: &Uuid) -> Result<()>;
}

/// Repository for time-series data.
#[async_trait]
pub trait TimeSeriesRepository: Send + Sync {
    /// Store a data point.
    async fn store(&self, data_point: &DataPoint) -> Result<()>;

    /// Store a batch of data points.
    async fn store_batch(&self, data_points: &[DataPoint]) -> Result<()>;

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

    /// Delete data points for a stream within a time range.
    async fn delete(
        &self,
        stream_id: &Uuid,
        start_time: &DateTime<Utc>,
        end_time: &DateTime<Utc>,
    ) -> Result<u64>;
}

/// Postgres implementation of the stream repository.
pub struct PgStreamRepository {
    /// Database connection pool
    pool: PgPool,
}

#[async_trait]
impl StreamRepository for PgStreamRepository {
    async fn create(&self, stream: &Stream) -> Result<()> {
        debug!("Creating stream: {}", stream.name);

        sqlx::query(
            r#"
            INSERT INTO streams (id, name, description, schema, retention_days, retention_max_records, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(stream.id)
        .bind(&stream.name)
        .bind(&stream.description)
        .bind(&stream.schema)
        .bind(stream.retention.days)
        .bind(stream.retention.max_records)
        .bind(stream.created_at)
        .bind(stream.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to create stream: {}", e)))?;

        Ok(())
    }

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

    async fn update(&self, stream: &Stream) -> Result<()> {
        debug!("Updating stream: {}", stream.id);

        sqlx::query(
            r#"
            UPDATE streams
            SET name = $1, description = $2, schema = $3, retention_days = $4, retention_max_records = $5, updated_at = $6
            WHERE id = $7
            "#,
        )
        .bind(&stream.name)
        .bind(&stream.description)
        .bind(&stream.schema)
        .bind(stream.retention.days)
        .bind(stream.retention.max_records)
        .bind(stream.updated_at)
        .bind(stream.id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to update stream: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        debug!("Deleting stream: {}", id);

        sqlx::query("DELETE FROM streams WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to delete stream: {}", e)))?;

        Ok(())
    }
}

/// Postgres implementation of the time-series repository.
pub struct PgTimeSeriesRepository {
    /// Database connection pool
    pool: PgPool,
}

#[async_trait]
impl TimeSeriesRepository for PgTimeSeriesRepository {
    async fn store(&self, data_point: &DataPoint) -> Result<()> {
        debug!("Storing data point for stream: {}", data_point.stream_id);

        sqlx::query(
            r#"
            INSERT INTO data_points (id, stream_id, timestamp, payload)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(data_point.id)
        .bind(data_point.stream_id)
        .bind(data_point.timestamp)
        .bind(&data_point.payload)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to store data point: {}", e)))?;

        Ok(())
    }

    async fn store_batch(&self, data_points: &[DataPoint]) -> Result<()> {
        if data_points.is_empty() {
            return Ok(());
        }

        debug!("Storing batch of {} data points", data_points.len());

        // Start a transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(format!("Failed to start transaction: {}", e)))?;

        for data_point in data_points {
            sqlx::query(
                r#"
                INSERT INTO data_points (id, stream_id, timestamp, payload)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(data_point.id)
            .bind(data_point.stream_id)
            .bind(data_point.timestamp)
            .bind(&data_point.payload)
            .execute(&mut tx)
            .await
            .map_err(|e| Error::Database(format!("Failed to store data point in batch: {}", e)))?;
        }

        // Commit the transaction
        tx.commit()
            .await
            .map_err(|e| Error::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

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
            let payload: serde_json::Value = row.get("payload");

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
            let payload: serde_json::Value = row.get("payload");

            data_points.push(DataPoint {
                id,
                stream_id,
                timestamp,
                payload,
            });
        }

        Ok(data_points)
    }

    async fn delete(
        &self,
        stream_id: &Uuid,
        start_time: &DateTime<Utc>,
        end_time: &DateTime<Utc>,
    ) -> Result<u64> {
        debug!(
            "Deleting data points for stream: {} from {} to {}",
            stream_id, start_time, end_time
        );

        let result = sqlx::query(
            r#"
            DELETE FROM data_points
            WHERE stream_id = $1 AND timestamp >= $2 AND timestamp <= $3
            "#,
        )
        .bind(stream_id)
        .bind(start_time)
        .bind(end_time)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to delete data points: {}", e)))?;

        Ok(result.rows_affected())
    }
}

/// Manager for all repositories.
#[derive(Clone)]
pub struct RepositoryManager {
    /// Stream repository
    stream_repo: Arc<dyn StreamRepository>,

    /// Pipeline repository
    pipeline_repo: Arc<dyn PipelineRepository>,

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

        // Run migrations if configured
        if config.migrations.run_migrations {
            info!("Running database migrations");

            sqlx::migrate::Migrator::new(std::path::Path::new(&config.migrations.migration_path))
                .await
                .map_err(|e| Error::Database(format!("Failed to create migrator: {}", e)))?
                .run(&pool)
                .await
                .map_err(|e| Error::Database(format!("Failed to run migrations: {}", e)))?;

            info!("Migrations completed successfully");
        }

        // Create repositories
        let stream_repo =
            Arc::new(PgStreamRepository { pool: pool.clone() }) as Arc<dyn StreamRepository>;

        // Pipeline repo is not implemented for MVP
        let pipeline_repo = Arc::new(DummyPipelineRepository {}) as Arc<dyn PipelineRepository>;

        let time_series_repo = Arc::new(PgTimeSeriesRepository { pool: pool.clone() })
            as Arc<dyn TimeSeriesRepository>;

        Ok(Self {
            stream_repo,
            pipeline_repo,
            time_series_repo,
        })
    }

    /// Get the stream repository.
    pub fn stream_repository(&self) -> Arc<dyn StreamRepository> {
        Arc::clone(&self.stream_repo)
    }

    /// Get the pipeline repository.
    pub fn pipeline_repository(&self) -> Arc<dyn PipelineRepository> {
        Arc::clone(&self.pipeline_repo)
    }

    /// Get the time-series repository.
    pub fn time_series_repository(&self) -> Arc<dyn TimeSeriesRepository> {
        Arc::clone(&self.time_series_repo)
    }
}

/// Dummy implementation of the pipeline repository for MVP.
struct DummyPipelineRepository;

#[async_trait]
impl PipelineRepository for DummyPipelineRepository {
    async fn create(&self, _pipeline: &Pipeline) -> Result<()> {
        Err(Error::Unexpected(
            "Pipeline repository not implemented for MVP".to_string(),
        ))
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Pipeline>> {
        Err(Error::Unexpected(
            "Pipeline repository not implemented for MVP".to_string(),
        ))
    }

    async fn list(&self) -> Result<Vec<Pipeline>> {
        Ok(vec![])
    }

    async fn list_for_stream(&self, _stream_id: &Uuid) -> Result<Vec<Pipeline>> {
        Ok(vec![])
    }

    async fn update(&self, _pipeline: &Pipeline) -> Result<()> {
        Err(Error::Unexpected(
            "Pipeline repository not implemented for MVP".to_string(),
        ))
    }

    async fn delete(&self, _id: &Uuid) -> Result<()> {
        Err(Error::Unexpected(
            "Pipeline repository not implemented for MVP".to_string(),
        ))
    }
}
