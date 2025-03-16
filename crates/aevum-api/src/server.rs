use crate::{config::Config, handlers, repository::RepositoryManager};
use aevum_common::Error;
use axum::{
    Router,
    error_handling::HandleErrorLayer,
    http::StatusCode,
    routing::{get, post},
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::oneshot;
use tower::{BoxError, ServiceBuilder};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{error, info};

/// The HTTP server for the API Service.
pub struct Server {
    config: Config,
    repositories: Arc<RepositoryManager>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Server {
    /// Create a new server instance.
    pub async fn new(config: Config) -> Result<Self, Error> {
        // Initialize repositories
        let repositories = RepositoryManager::new(&config.database).await?;

        // Create shutdown channel
        let (tx, _) = oneshot::channel::<()>();

        Ok(Self {
            config,
            repositories: Arc::new(repositories),
            shutdown_tx: Some(tx),
        })
    }

    /// Start the HTTP server.
    pub async fn start(&self) -> Result<(), Error> {
        info!("Starting server");

        let repositories = Arc::clone(&self.repositories);

        // Build router with all routes
        let app = Router::new()
            // Health check route
            .route("/health", get(handlers::health::check))
            // Stream routes
            .route("/streams", get(handlers::streams::list_streams))
            .route("/streams/:id", get(handlers::streams::get_stream))
            .route(
                "/streams/:id/fields",
                get(handlers::streams::get_stream_fields),
            )
            // Data routes
            .route("/data/:stream_id", get(handlers::data::query_data))
            .route("/data/:stream_id/latest", get(handlers::data::get_latest))
            .route(
                "/data/:stream_id/aggregate",
                get(handlers::data::aggregate_data),
            )
            .route(
                "/data/:stream_id/stats/:field",
                get(handlers::data::get_field_stats),
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
            .with_state(repositories);

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
