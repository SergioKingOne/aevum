use aevum_storage::{config::Config, consumer::KafkaConsumer, repository::RepositoryManager};
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    info!("Starting Aevum Storage Service");

    // Load configuration
    info!("Loading configuration");
    let config = Config::load()?;
    info!("Configuration loaded successfully");

    // Connect to database
    info!("Connecting to database");
    let repository_manager = RepositoryManager::new(&config.database).await?;
    info!("Database connection established");

    // Initialize Kafka consumer
    info!("Initializing Kafka consumer");
    let consumer = KafkaConsumer::new(&config.kafka, repository_manager.clone()).await?;
    info!("Kafka consumer initialized");

    // Start consumer
    let consumer = Arc::new(consumer);
    let consumer_clone = Arc::clone(&consumer);

    // Handle shutdown gracefully
    tokio::spawn(async move {
        let ctrl_c = tokio::signal::ctrl_c();

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C signal");
            },
            _ = terminate => {
                info!("Received SIGTERM signal");
            }
        }

        info!("Shutting down gracefully");
        consumer_clone.stop().await;
    });

    // Start consuming messages
    if let Err(e) = consumer.start().await {
        error!("Consumer error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
