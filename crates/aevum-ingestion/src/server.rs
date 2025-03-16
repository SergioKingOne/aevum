use crate::{config::Config, handlers, producer::KafkaProducer};
use aevum_common::Error;
use axum::{
    Router,
    error_handling::HandleErrorLayer,
    extract::Extension,
    http::StatusCode,
    routing::{delete, get, post},
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::oneshot;
use tower::{BoxError, ServiceBuilder};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{error, info};

/// The HTTP server for the Ingestion Service.
pub struct Server {
    config: Config,
    producer: Arc<KafkaProducer>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Server {
    /// Create a new server instance.
    pub async fn new(config: Config) -> Result<Self, Error> {
        // Initialize Kafka producer
        let producer = KafkaProducer::new(&config.kafka)
            .await
            .map_err(|e| Error::Unexpected(format!("Failed to create Kafka producer: {}", e)))?;

        Ok(Self {
            config,
            producer: Arc::new(producer),
            shutdown_tx: None,
        })
    }

    /// Start the HTTP server.
    pub async fn start(&self) -> Result<(), Error> {
        let producer = Arc::clone(&self.producer);

        // Build router with all routes
        let app = Router::new()
            // Health check route
            .route("/health", get(handlers::health::check))
            // Stream management routes
            .route("/streams", get(handlers::streams::list_streams))
            .route("/streams", post(handlers::streams::create_stream))
            .route("/streams/:id", get(handlers::streams::get_stream))
            .route("/streams/:id", post(handlers::streams::update_stream))
            .route("/streams/:id", delete(handlers::streams::delete_stream))
            // Data ingestion routes
            .route("/ingest/:stream_id", post(handlers::ingest::ingest_data))
            .route(
                "/ingest/:stream_id/batch",
                post(handlers::ingest::ingest_batch),
            )
            // Add middleware
            .layer(
                ServiceBuilder::new()
                    // Add error handling
                    .layer(HandleErrorLayer::new(|e: BoxError| async move {
                        error!("Request failed: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    }))
                    // Add timeout
                    .timeout(Duration::from_secs(self.config.server.timeout_seconds))
                    // Add tracing
                    .layer(TraceLayer::new_for_http())
                    // Add CORS
                    .layer(
                        CorsLayer::new()
                            .allow_origin(Any)
                            .allow_methods(Any)
                            .allow_headers(Any),
                    ),
            )
            // Add state
            .with_state(producer);

        // Get address to bind to
        let addr = format!("{}:{}", self.config.server.host, self.config.server.port)
            .parse::<SocketAddr>()
            .map_err(|e| Error::Config(format!("Invalid server address: {}", e)))?;

        info!("Listening on {}", addr);

        // Create shutdown channel
        let (tx, rx) = oneshot::channel::<()>();

        // Store sender for shutdown
        let this = unsafe {
            // This is safe because we're making the mutable access exclusive
            (self as *const Self as *mut Self).as_mut().unwrap()
        };
        this.shutdown_tx = Some(tx);

        // Start server
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(async {
                rx.await.ok();
                info!("Server shutting down");
            })
            .await
            .map_err(|e| Error::Unexpected(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// Gracefully shut down the server.
    pub async fn shutdown(&self) {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
            info!("Shutdown signal sent");
        }
    }
}
