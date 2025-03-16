use crate::{config::Config, consumer::KafkaConsumer, pipeline::PipelineManager};
use aevum_common::{
    Error, Result,
    models::{DataPoint, Pipeline},
};
use rdkafka::{
    ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, mpsc, oneshot},
    time,
};
use tracing::{error, info, warn};

/// Stream processor that consumes data points, applies pipelines, and produces results.
pub struct StreamProcessor {
    /// Configuration
    config: Config,

    /// Kafka consumer
    consumer: Arc<KafkaConsumer>,

    /// Kafka producer for processed data
    producer: FutureProducer,

    /// Pipeline manager
    pipeline_manager: Arc<Mutex<PipelineManager>>,

    /// Channel for receiving shutdown signals
    shutdown_rx: Option<oneshot::Receiver<()>>,

    /// Channel for sending shutdown signals
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl StreamProcessor {
    /// Create a new stream processor.
    pub async fn new(config: Config) -> Result<Self> {
        // Initialize Kafka consumer
        let consumer = KafkaConsumer::new(&config.kafka).await?;

        // Initialize Kafka producer
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.kafka.brokers)
            .set("client.id", &format!("{}-producer", config.kafka.client_id))
            .set("message.timeout.ms", "5000")
            .create()
            .map_err(|e| {
                Error::ExternalService(format!("Failed to create Kafka producer: {}", e))
            })?;

        // Create pipeline manager
        let pipeline_manager = PipelineManager::new();

        // Create shutdown channel
        let (tx, rx) = oneshot::channel::<()>();

        Ok(Self {
            config,
            consumer: Arc::new(consumer),
            producer,
            pipeline_manager: Arc::new(Mutex::new(pipeline_manager)),
            shutdown_rx: Some(rx),
            shutdown_tx: Some(tx),
        })
    }

    /// Register a pipeline.
    pub async fn register_pipeline(&self, pipeline: Pipeline) -> Result<()> {
        let mut manager = self.pipeline_manager.lock().await;
        manager.register_pipeline(pipeline)
    }

    /// Start the processor.
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting stream processor");

        // Create channel for data points
        let (tx, mut rx) = mpsc::channel::<DataPoint>(self.config.processing.batch_size * 2);

        // Start the consumer
        let consumer_clone = Arc::clone(&self.consumer);
        tokio::spawn(async move {
            if let Err(e) = consumer_clone.consume_data_points(tx).await {
                error!("Consumer error: {}", e);
            }
        });

        // Start the processing loop
        let pipeline_manager = Arc::clone(&self.pipeline_manager);
        let producer = self.producer.clone();
        let processed_topic = self.config.kafka.topics.processed_data.clone();
        let batch_size = self.config.processing.batch_size;
        let interval_ms = self.config.processing.interval_ms;

        let mut shutdown_rx = self
            .shutdown_rx
            .take()
            .ok_or_else(|| Error::Unexpected("Shutdown receiver already taken".to_string()))?;

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(interval_ms));
            let mut batch = Vec::with_capacity(batch_size);

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        info!("Processor received shutdown signal");
                        break;
                    }
                    _ = interval.tick() => {
                        // Process any accumulated batch
                        if !batch.is_empty() {
                            let points_to_process = std::mem::take(&mut batch);
                            let manager = pipeline_manager.lock().await;

                            match manager.process_batch(points_to_process).await {
                                Ok(processed) => {
                                    if !processed.is_empty() {
                                        if let Err(e) = send_processed_data(&producer, &processed_topic, &processed).await {
                                            error!("Failed to send processed data: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Error processing batch: {}", e);
                                }
                            }
                        }
                    }
                    Some(data_point) = rx.recv() => {
                        batch.push(data_point);

                        // If batch is full, process it
                        if batch.len() >= batch_size {
                            let points_to_process = std::mem::take(&mut batch);
                            let manager = pipeline_manager.lock().await;

                            match manager.process_batch(points_to_process).await {
                                Ok(processed) => {
                                    if !processed.is_empty() {
                                        if let Err(e) = send_processed_data(&producer, &processed_topic, &processed).await {
                                            error!("Failed to send processed data: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Error processing batch: {}", e);
                                }
                            }
                        }
                    }
                }
            }

            info!("Processing loop terminated");
        });

        info!("Processor started");
        Ok(())
    }

    /// Shutdown the processor.
    pub async fn shutdown(&self) {
        info!("Shutting down processor");

        // Send shutdown signal
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }

        // Stop the consumer
        if let Err(e) = self.consumer.stop().await {
            error!("Error stopping consumer: {}", e);
        }

        info!("Processor shutdown complete");
    }
}

/// Send processed data points to Kafka.
async fn send_processed_data(
    producer: &FutureProducer,
    topic: &str,
    data_points: &[DataPoint],
) -> Result<()> {
    for data_point in data_points {
        let key = data_point.id.to_string();
        let payload = serde_json::to_string(data_point).map_err(|e| Error::Serialization(e))?;

        let record = FutureRecord::to(topic).key(&key).payload(&payload);

        match producer.send(record, Duration::from_secs(5)).await {
            Ok(_) => {}
            Err((e, _)) => {
                warn!("Failed to send processed data point: {}", e);
            }
        }
    }

    Ok(())
}
