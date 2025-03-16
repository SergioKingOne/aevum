use aevum_processor::{config::Config, processor::StreamProcessor};
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    info!("Starting Aevum Processor Service");

    // Load configuration
    info!("Loading configuration");
    let config = Config::load()?;
    info!("Configuration loaded successfully");

    // Create and start the processor
    let processor = Arc::new(StreamProcessor::new(config).await?);
    info!("Processor initialized");

    // Handle shutdown gracefully
    let processor_clone = Arc::clone(&processor);
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
        processor_clone.shutdown().await;
    });

    // Start the processor
    if let Err(e) = processor.start().await {
        error!("Processor error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
