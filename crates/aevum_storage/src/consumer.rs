use crate::{config::KafkaConfig, repository::RepositoryManager};
use aevum_common::{
    Error, Result,
    models::{DataPoint, Stream},
};
use futures::stream::StreamExt;
use rdkafka::{
    ClientConfig, Message,
    consumer::{Consumer, StreamConsumer},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::time;
use tracing::{debug, error, info, warn};

/// Kafka consumer for the Storage Service.
pub struct KafkaConsumer {
    /// Kafka configuration
    config: KafkaConfig,

    /// Repository manager
    repositories: RepositoryManager,

    /// Running flag
    running: Arc<AtomicBool>,
}

impl KafkaConsumer {
    /// Create a new Kafka consumer.
    pub async fn new(config: &KafkaConfig, repositories: RepositoryManager) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            repositories,
            running: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Start consuming messages from Kafka.
    pub async fn start(&self) -> Result<()> {
        info!("Starting Kafka consumer");

        // Set running flag
        self.running.store(true, Ordering::SeqCst);

        // Create consumer
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &self.config.brokers)
            .set("group.id", &self.config.group_id)
            .set("client.id", &self.config.client_id)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .set("enable.partition.eof", "false")
            .create()
            .map_err(|e| {
                Error::ExternalService(format!("Failed to create Kafka consumer: {}", e))
            })?;

        // Subscribe to topics
        let topics = [
            &self.config.topics.raw_data,
            &self.config.topics.processed_data,
            &self.config.topics.stream_metadata,
        ];

        consumer
            .subscribe(&topics)
            .map_err(|e| Error::ExternalService(format!("Failed to subscribe to topics: {}", e)))?;

        info!("Subscribed to topics: {:?}", topics);

        // Start message processing loop
        let mut message_stream = consumer.stream();
        let mut batch = Vec::with_capacity(100);
        let mut last_commit = time::Instant::now();

        while self.running.load(Ordering::SeqCst) {
            tokio::select! {
                Some(message_result) = message_stream.next() => {
                    match message_result {
                        Ok(message) => {
                            let topic = message.topic();

                            if let Some(payload) = message.payload() {
                                if topic == &self.config.topics.raw_data || topic == &self.config.topics.processed_data {
                                    match self.process_data_point(payload).await {
                                        Ok(data_point) => {
                                            batch.push(data_point);

                                            // Flush batch if it's full
                                            if batch.len() >= 100 {
                                                self.flush_batch(&mut batch).await;
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to process data point: {}", e);
                                        }
                                    }
                                } else if topic == &self.config.topics.stream_metadata {
                                    match self.process_stream_metadata(payload).await {
                                        Ok(_) => {
                                            debug!("Processed stream metadata");
                                        }
                                        Err(e) => {
                                            error!("Failed to process stream metadata: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error receiving message: {}", e);
                        }
                    }
                }
                _ = time::sleep(Duration::from_secs(1)) => {
                    // Flush batch if it's not empty and 5 seconds have passed
                    if !batch.is_empty() && last_commit.elapsed() >= Duration::from_secs(5) {
                        self.flush_batch(&mut batch).await;
                        last_commit = time::Instant::now();
                    }
                }
            }
        }

        // Flush any remaining data
        if !batch.is_empty() {
            self.flush_batch(&mut batch).await;
        }

        // Unsubscribe
        consumer
            .unsubscribe()
            .map_err(|e| Error::ExternalService(format!("Failed to unsubscribe: {}", e)))?;

        info!("Kafka consumer stopped");
        Ok(())
    }

    /// Process a data point message.
    async fn process_data_point(&self, payload: &[u8]) -> Result<DataPoint> {
        let data_point: DataPoint =
            serde_json::from_slice(payload).map_err(|e| Error::Serialization(e))?;

        debug!("Received data point for stream: {}", data_point.stream_id);

        Ok(data_point)
    }

    /// Process a stream metadata message.
    async fn process_stream_metadata(&self, payload: &[u8]) -> Result<()> {
        let stream: Stream =
            serde_json::from_slice(payload).map_err(|e| Error::Serialization(e))?;

        debug!("Received stream metadata: {}", stream.id);

        // Check if stream exists
        let repo = self.repositories.stream_repository();

        match repo.get(&stream.id).await? {
            Some(_) => {
                // Update existing stream
                repo.update(&stream).await?;
                debug!("Updated stream: {}", stream.id);
            }
            None => {
                // Create new stream
                repo.create(&stream).await?;
                debug!("Created stream: {}", stream.id);
            }
        }

        Ok(())
    }

    /// Flush a batch of data points to storage.
    async fn flush_batch(&self, batch: &mut Vec<DataPoint>) {
        if batch.is_empty() {
            return;
        }

        debug!("Flushing batch of {} data points", batch.len());

        let repo = self.repositories.time_series_repository();

        match repo.store_batch(batch).await {
            Ok(_) => {
                debug!("Stored batch of {} data points", batch.len());
            }
            Err(e) => {
                error!("Failed to store batch: {}", e);
            }
        }

        batch.clear();
    }

    /// Stop the consumer.
    pub async fn stop(&self) {
        info!("Stopping Kafka consumer");
        self.running.store(false, Ordering::SeqCst);
    }
}
