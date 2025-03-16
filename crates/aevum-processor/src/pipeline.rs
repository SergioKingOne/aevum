use crate::operations::{OperationFactory, StreamOperation};
use aevum_common::{
    Result,
    models::{DataPoint, Pipeline},
};
use std::sync::Arc;
use tracing::{debug, info};

/// Executes a processing pipeline on data points.
pub struct PipelineExecutor {
    /// The pipeline configuration
    pipeline: Pipeline,

    /// The operations to apply in sequence
    operations: Vec<Box<dyn StreamOperation>>,
}

impl PipelineExecutor {
    /// Create a new pipeline executor.
    pub fn new(pipeline: Pipeline) -> Result<Self> {
        info!("Creating pipeline executor for: {}", pipeline.name);

        let mut operations = Vec::with_capacity(pipeline.operations.len());

        // Create operations from pipeline configuration
        for operation in &pipeline.operations {
            let op = OperationFactory::create(operation.clone(), pipeline.source_stream_id)?;
            operations.push(op);
        }

        Ok(Self {
            pipeline,
            operations,
        })
    }

    /// Process a single data point through the pipeline.
    pub async fn process(&self, data_point: DataPoint) -> Result<Vec<DataPoint>> {
        debug!(
            "Processing data point through pipeline: {}",
            self.pipeline.name
        );

        // Check if this data point belongs to the source stream
        if data_point.stream_id != self.pipeline.source_stream_id {
            debug!("Skipping data point from unrelated stream");
            return Ok(vec![]);
        }

        // Initialize with the original data point
        let mut current = vec![data_point];

        // Apply each operation in sequence
        for operation in &self.operations {
            // Process current batch
            let mut next = Vec::new();

            for point in current {
                let processed = operation.process_single(point).await?;
                if let Some(p) = processed {
                    next.push(p);
                }
            }

            // Update current batch for next operation
            current = next;

            // If we filtered out all data points, stop processing
            if current.is_empty() {
                break;
            }
        }

        // If a destination stream is specified, update the stream ID
        if let Some(dest_id) = self.pipeline.destination_stream_id {
            for point in &mut current {
                point.stream_id = dest_id;
            }
        }

        Ok(current)
    }

    /// Process a batch of data points through the pipeline.
    pub async fn process_batch(&self, data_points: Vec<DataPoint>) -> Result<Vec<DataPoint>> {
        if data_points.is_empty() {
            return Ok(vec![]);
        }

        info!(
            "Processing batch of {} data points through pipeline: {}",
            data_points.len(),
            self.pipeline.name
        );

        // Filter to only points from the source stream
        let mut current: Vec<DataPoint> = data_points
            .into_iter()
            .filter(|p| p.stream_id == self.pipeline.source_stream_id)
            .collect();

        if current.is_empty() {
            return Ok(vec![]);
        }

        // Apply each operation in sequence
        for operation in &self.operations {
            // Process current batch
            current = operation.process_batch(current).await?;

            // If we filtered out all data points, stop processing
            if current.is_empty() {
                break;
            }
        }

        // If a destination stream is specified, update the stream ID
        if let Some(dest_id) = self.pipeline.destination_stream_id {
            for point in &mut current {
                point.stream_id = dest_id;
            }
        }

        Ok(current)
    }
}

/// Manages multiple pipeline executors.
pub struct PipelineManager {
    /// Registered pipelines
    pipelines: Vec<Arc<PipelineExecutor>>,
}

impl PipelineManager {
    /// Create a new pipeline manager.
    pub fn new() -> Self {
        Self {
            pipelines: Vec::new(),
        }
    }

    /// Register a new pipeline.
    pub fn register_pipeline(&mut self, pipeline: Pipeline) -> Result<()> {
        info!("Registering pipeline: {}", pipeline.name);

        let executor = PipelineExecutor::new(pipeline)?;
        self.pipelines.push(Arc::new(executor));

        Ok(())
    }

    /// Process a data point through all relevant pipelines.
    pub async fn process(&self, data_point: DataPoint) -> Result<Vec<DataPoint>> {
        let stream_id = data_point.stream_id;
        debug!(
            "Processing data point through pipelines for stream: {}",
            stream_id
        );

        let mut results = Vec::new();

        // Find pipelines that match this stream
        for pipeline in &self.pipelines {
            if pipeline.pipeline.source_stream_id == stream_id {
                let processed = pipeline.process(data_point.clone()).await?;
                results.extend(processed);
            }
        }

        Ok(results)
    }

    /// Process a batch of data points through all relevant pipelines.
    pub async fn process_batch(&self, data_points: Vec<DataPoint>) -> Result<Vec<DataPoint>> {
        if data_points.is_empty() {
            return Ok(vec![]);
        }

        info!(
            "Processing batch of {} data points through pipelines",
            data_points.len()
        );

        let mut results = Vec::new();

        // Group data points by stream
        let mut by_stream: std::collections::HashMap<uuid::Uuid, Vec<DataPoint>> =
            std::collections::HashMap::new();

        for point in data_points {
            by_stream.entry(point.stream_id).or_default().push(point);
        }

        // Process each stream's data points through relevant pipelines
        for (stream_id, points) in by_stream {
            // Find pipelines that match this stream
            for pipeline in &self.pipelines {
                if pipeline.pipeline.source_stream_id == stream_id {
                    let processed = pipeline.process_batch(points.clone()).await?;
                    results.extend(processed);
                }
            }
        }

        Ok(results)
    }
}
