use bon::Builder;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a data stream configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
pub struct Stream {
    /// Unique identifier for the stream
    pub id: Uuid,

    /// Human-readable name for the stream
    pub name: String,

    /// Optional description of the stream
    pub description: Option<String>,

    /// Schema definition for data in this stream (JSON Schema format)
    pub schema: serde_json::Value,

    /// Configuration for data retention
    #[builder(default)]
    pub retention: RetentionConfig,

    /// When the stream was created
    #[builder(default = Utc::now())]
    pub created_at: DateTime<Utc>,

    /// When the stream was last updated
    #[builder(default = Utc::now())]
    pub updated_at: DateTime<Utc>,
}

/// Configuration for data retention policies.
#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
pub struct RetentionConfig {
    /// How long to keep data in days
    #[builder(default = default_retention_days())]
    pub days: u32,

    /// Maximum number of records to keep
    pub max_records: Option<u64>,
}

fn default_retention_days() -> u32 {
    30
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Represents a data point in a stream.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataPoint {
    /// The stream this data belongs to
    pub stream_id: Uuid,

    /// Unique identifier for this data point
    pub id: Uuid,

    /// Timestamp when this data was generated
    pub timestamp: DateTime<Utc>,

    /// The actual data payload
    pub payload: serde_json::Value,
}

/// Represents a processing pipeline configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
pub struct Pipeline {
    /// Unique identifier for the pipeline
    pub id: Uuid,

    /// Human-readable name for the pipeline
    pub name: String,

    /// Optional description of the pipeline
    pub description: Option<String>,

    /// Source stream for this pipeline
    pub source_stream_id: Uuid,

    /// Optional destination stream for processed data
    pub destination_stream_id: Option<Uuid>,

    /// Processing operations to apply
    pub operations: Vec<Operation>,

    /// When the pipeline was created
    #[builder(default = Utc::now())]
    pub created_at: DateTime<Utc>,

    /// When the pipeline was last updated
    #[builder(default = Utc::now())]
    pub updated_at: DateTime<Utc>,
}

/// Represents a single processing operation in a pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Operation {
    /// Filter data points based on a condition
    Filter { condition: String },

    /// Transform data points using a JSON transformation
    Transform { transformation: String },

    /// Aggregate data points over a window
    Aggregate {
        window_seconds: u32,
        function: AggregationFunction,
        field: String,
    },

    /// Detect anomalies in the data stream
    AnomalyDetection {
        algorithm: String,
        parameters: serde_json::Value,
    },
}

/// Supported aggregation functions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AggregationFunction {
    Sum,
    Average,
    Min,
    Max,
    Count,
}
