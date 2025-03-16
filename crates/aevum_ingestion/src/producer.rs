use crate::config::KafkaConfig;
use aevum_common::{Error, Result, models::DataPoint};
use rdkafka::{
    ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use serde::Serialize;
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

/// Kafka producer for sending data to topics.
pub struct KafkaProducer {
    /// The underlying Kafka producer
    producer: FutureProducer,

    /// Topic configuration
    topics: Topics,
}

/// Topic names used by the producer.
struct Topics {
    /// Topic for raw data ingestion
    raw_data: String,

    /// Topic for stream metadata
    stream_metadata: String,
}

impl KafkaProducer {
    /// Create a new Kafka producer.
    pub async fn new(config: &KafkaConfig) -> Result<Self> {
        info!("Initializing Kafka producer");

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("client.id", &config.client_id)
            .set("message.timeout.ms", "5000")
            .set("acks", "all")
            .create()
            .map_err(|e| {
                Error::ExternalService(format!("Failed to create Kafka producer: {}", e))
            })?;

        let topics = Topics {
            raw_data: config.topics.raw_data.clone(),
            stream_metadata: config.topics.stream_metadata.clone(),
        };

        Ok(Self { producer, topics })
    }

    /// Send a data point to the raw data topic.
    pub async fn send_data_point(&self, data_point: &DataPoint) -> Result<()> {
        self.send_to_topic(
            &self.topics.raw_data,
            &data_point.id.to_string(),
            data_point,
        )
        .await
    }

    /// Send a batch of data points to the raw data topic.
    pub async fn send_data_points(&self, data_points: &[DataPoint]) -> Result<()> {
        for data_point in data_points {
            if let Err(err) = self.send_data_point(data_point).await {
                error!("Failed to send data point: {}", err);
                return Err(err);
            }
        }

        Ok(())
    }

    /// Send a stream metadata update.
    pub async fn send_stream_metadata<T: Serialize>(
        &self,
        stream_id: &Uuid,
        metadata: &T,
    ) -> Result<()> {
        self.send_to_topic(
            &self.topics.stream_metadata,
            &stream_id.to_string(),
            metadata,
        )
        .await
    }

    /// Send a message to a specific topic.
    async fn send_to_topic<T: Serialize>(&self, topic: &str, key: &str, payload: &T) -> Result<()> {
        let payload = serde_json::to_string(payload).map_err(|e| Error::Serialization(e))?;

        let record = FutureRecord::to(topic).key(key).payload(&payload);

        self.producer
            .send(record, Duration::from_secs(5))
            .await
            .map_err(|(e, _)| {
                Error::ExternalService(format!("Failed to send message to Kafka: {}", e))
            })?;

        Ok(())
    }
}
