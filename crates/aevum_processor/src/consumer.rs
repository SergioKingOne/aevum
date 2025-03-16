use crate::config::KafkaConfig;
use aevum_common::{Error, Result, models::DataPoint};
use futures::stream::StreamExt;
use rdkafka::{
    ClientConfig, Message, TopicPartitionList,
    consumer::{Consumer, StreamConsumer},
};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Kafka consumer for receiving data from topics.
pub struct KafkaConsumer {
    /// The underlying Kafka consumer
    consumer: StreamConsumer,

    /// Topic for raw data consumption
    raw_data_topic: String,

    /// Topic for stream metadata
    stream_metadata_topic: String,

    /// Topic for pipeline configurations
    pipelines_topic: String,
}

impl KafkaConsumer {
    /// Create a new Kafka consumer.
    pub async fn new(config: &KafkaConfig) -> Result<Self> {
        info!("Initializing Kafka consumer");

        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("group.id", &config.group_id)
            .set("client.id", &config.client_id)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .set("enable.partition.eof", "false")
            .create()
            .map_err(|e| {
                Error::ExternalService(format!("Failed to create Kafka consumer: {}", e))
            })?;

        let topics = [
            &config.topics.raw_data,
            &config.topics.stream_metadata,
            &config.topics.pipelines,
        ];

        // Subscribe to topics
        consumer
            .subscribe(&topics)
            .map_err(|e| Error::ExternalService(format!("Failed to subscribe to topics: {}", e)))?;

        info!("Subscribed to topics: {:?}", topics);

        Ok(Self {
            consumer,
            raw_data_topic: config.topics.raw_data.clone(),
            stream_metadata_topic: config.topics.stream_metadata.clone(),
            pipelines_topic: config.topics.pipelines.clone(),
        })
    }

    /// Start consuming data points and sending them to the provided channel.
    pub async fn consume_data_points(
        self: Arc<Self>,
        sender: mpsc::Sender<DataPoint>,
    ) -> Result<()> {
        info!("Starting to consume data points");

        let mut message_stream = self.consumer.stream();

        while let Some(message_result) = message_stream.next().await {
            match message_result {
                Ok(message) => {
                    let topic = message.topic();

                    if topic == self.raw_data_topic {
                        if let Some(payload) = message.payload() {
                            match self.parse_message::<DataPoint>(payload) {
                                Ok(data_point) => {
                                    if let Err(e) = sender.send(data_point).await {
                                        error!("Failed to send data point to channel: {}", e);
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to parse data point: {}", e);
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

        warn!("Message stream ended");
        Ok(())
    }

    /// Parse a message payload into the specified type.
    fn parse_message<T: DeserializeOwned>(&self, payload: &[u8]) -> Result<T> {
        serde_json::from_slice(payload).map_err(|e| Error::Serialization(e))
    }

    /// Commit offsets for the given topic partition list.
    pub async fn commit_offsets(&self, tpl: &TopicPartitionList) -> Result<()> {
        self.consumer
            .commit(tpl, rdkafka::consumer::CommitMode::Async)
            .map_err(|e| Error::ExternalService(format!("Failed to commit offsets: {}", e)))?;

        Ok(())
    }

    /// Stop consuming messages.
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Kafka consumer");

        // Unsubscribe from all topics
        self.consumer
            .unsubscribe()
            .map_err(|e| Error::ExternalService(format!("Failed to unsubscribe: {}", e)))?;

        Ok(())
    }
}
