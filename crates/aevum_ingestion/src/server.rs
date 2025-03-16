use crate::{config::Config, handlers, producer::KafkaProducer};
use actix_cors::Cors;
use actix_web::{App, HttpServer, dev::Server as ActixServer, middleware, web};
use aevum_common::Error;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::oneshot;
use tracing::info;

/// The HTTP server for the Ingestion Service.
pub struct Server {
    config: Config,
    producer: Arc<KafkaProducer>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    server: Option<ActixServer>,
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
            server: None,
        })
    }

    /// Start the HTTP server.
    pub async fn start(&mut self) -> Result<(), Error> {
        let producer = Arc::clone(&self.producer);

        // Get address to bind to
        let addr = format!("{}:{}", self.config.server.host, self.config.server.port)
            .parse::<SocketAddr>()
            .map_err(|e| Error::Config(format!("Invalid server address: {}", e)))?;

        info!("Listening on {}", addr);

        // Create shutdown channel
        let (tx, rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(tx);

        // Build and start the server
        let server = HttpServer::new(move || {
            // Configure CORS
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header();

            App::new()
                // Configure state
                .app_data(web::Data::new(producer.clone()))
                // Configure middlewares
                .wrap(middleware::Logger::default())
                // We'll handle timeouts at a different level - add version header for now
                .wrap(
                    middleware::DefaultHeaders::new().add(("X-Version", env!("CARGO_PKG_VERSION"))),
                )
                .wrap(cors)
                // Configure routes
                // Health check route
                .route("/health", web::get().to(handlers::health::check))
                // Stream management routes
                .route("/streams", web::get().to(handlers::streams::list_streams))
                .route("/streams", web::post().to(handlers::streams::create_stream))
                .route(
                    "/streams/{id}",
                    web::get().to(handlers::streams::get_stream),
                )
                // Data ingestion routes
                .route(
                    "/ingest/{stream_id}",
                    web::post().to(handlers::ingest::ingest_data),
                )
                .route(
                    "/ingest/{stream_id}/batch",
                    web::post().to(handlers::ingest::ingest_batch),
                )
        })
        .bind(addr)
        .map_err(|e| Error::Unexpected(format!("Failed to bind to address: {}", e)))?
        .run();

        // Store server instance
        self.server = Some(server);

        // Extract server handle to await in another task
        let server_handle = self.server.as_ref().unwrap().handle();

        // Spawn a task to wait for shutdown signal
        tokio::spawn(async move {
            // Wait for shutdown signal
            let _ = rx.await;
            info!("Server shutting down");
            // Stop server gracefully
            server_handle.stop(true).await;
        });

        // Start server - take ownership of the server to await it
        if let Some(server) = self.server.take() {
            server
                .await
                .map_err(|e| Error::Unexpected(format!("Server error: {}", e)))?;
        }

        Ok(())
    }

    /// Gracefully shut down the server.
    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
            info!("Shutdown signal sent");
        }
    }
}
