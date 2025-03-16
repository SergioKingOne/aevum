use aevum_common::Error;
use bon::builder;
use config::{Config as ConfigLib, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for the API Service.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// HTTP server configuration
    #[builder(default = "default_server_config()")]
    pub server: ServerConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// CORS configuration
    #[builder(default = "default_cors_config()")]
    pub cors: CorsConfig,
}

/// HTTP server configuration.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to
    #[builder(default = "default_host()")]
    pub host: String,

    /// Port to listen on
    #[builder(default = "default_port()")]
    pub port: u16,

    /// Request timeout in seconds
    #[builder(default = "default_timeout()")]
    pub timeout_seconds: u64,
}

/// Database configuration.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,

    /// Maximum connections in the pool
    #[builder(default = "default_max_connections()")]
    pub max_connections: u32,

    /// Minimum connections in the pool
    #[builder(default = "default_min_connections()")]
    pub min_connections: u32,

    /// Connection timeout in seconds
    #[builder(default = "default_connect_timeout()")]
    pub connect_timeout_seconds: u64,
}

/// CORS configuration.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Whether to enable CORS
    #[builder(default = "default_cors_enabled()")]
    pub enabled: bool,

    /// Allowed origins
    #[builder(default = "default_allowed_origins()")]
    pub allowed_origins: Vec<String>,

    /// Whether to allow credentials
    #[builder(default = "default_allow_credentials()")]
    pub allow_credentials: bool,
}

fn default_server_config() -> ServerConfig {
    ServerConfigBuilder::new()
        .build()
        .expect("Default server config should be valid")
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_timeout() -> u64 {
    30
}

fn default_max_connections() -> u32 {
    10
}

fn default_min_connections() -> u32 {
    2
}

fn default_connect_timeout() -> u64 {
    10
}

fn default_cors_config() -> CorsConfig {
    CorsConfigBuilder::new()
        .build()
        .expect("Default CORS config should be valid")
}

fn default_cors_enabled() -> bool {
    true
}

fn default_allowed_origins() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_allow_credentials() -> bool {
    false
}

impl Config {
    /// Load configuration from files and environment variables.
    pub fn load() -> Result<Self, Error> {
        // Start with default configuration
        let mut config_builder = ConfigLib::builder();

        // Layer on configuration from files
        let config_dir = std::env::var("CONFIG_DIR").unwrap_or_else(|_| "./config".to_string());

        // Base configuration
        let base_config = Path::new(&config_dir).join("base.toml");
        if base_config.exists() {
            config_builder = config_builder.add_source(File::from(base_config));
        }

        // Environment-specific configuration
        let env = std::env::var("RUN_ENV").unwrap_or_else(|_| "development".to_string());
        let env_config = Path::new(&config_dir).join(format!("{}.toml", env));
        if env_config.exists() {
            config_builder = config_builder.add_source(File::from(env_config));
        }

        // Local overrides (gitignored)
        let local_config = Path::new(&config_dir).join("local.toml");
        if local_config.exists() {
            config_builder = config_builder.add_source(File::from(local_config));
        }

        // Add environment variables prefixed with AEVUM_API_
        config_builder = config_builder.add_source(
            Environment::with_prefix("AEVUM_API")
                .separator("__")
                .try_parsing(true),
        );

        // Build the configuration
        let config = config_builder
            .build()
            .map_err(|e: ConfigError| Error::Config(e.to_string()))?;

        // Deserialize to our Config struct
        let config: Config = config
            .try_deserialize()
            .map_err(|e| Error::Config(e.to_string()))?;

        Ok(config)
    }
}
