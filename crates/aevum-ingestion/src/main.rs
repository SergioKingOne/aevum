use aevum_ingestion::{config::Config, server::Server};
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    info!("Starting Aevum Ingestion Service");

    // Load configuration
    info!("Loading configuration");
    let config = Config::load()?;
    info!("Configuration loaded successfully");

    // Create and start the server
    let server = Arc::new(Server::new(config).await?);
    info!("Server initialized");

    // Handle shutdown gracefully
    let server_clone = Arc::clone(&server);
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
        server_clone.shutdown().await;
    });

    // Start the server
    if let Err(e) = server.start().await {
        error!("Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
