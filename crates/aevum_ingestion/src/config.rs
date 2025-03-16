use aevum_common::Error;
use bon::Builder;
use config::{Config as ConfigLib, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for the Ingestion Service.
#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
pub struct Config {
    /// HTTP server configuration
    #[builder(default)]
    pub server: ServerConfig,

    /// Kafka configuration
    pub kafka: KafkaConfig,

    /// Redis configuration for rate limiting and caching
    pub redis: Option<RedisConfig>,
}

/// HTTP server configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
pub struct ServerConfig {
    /// Host to bind to
    #[builder(default = default_host())]
    pub host: String,

    /// Port to listen on
    #[builder(default = default_port())]
    pub port: u16,
}

/// Kafka configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
pub struct KafkaConfig {
    /// Comma-separated list of broker addresses
    pub brokers: String,

    /// Client ID for Kafka connection
    #[builder(default = default_client_id())]
    pub client_id: String,

    /// Kafka topics
    pub topics: KafkaTopics,
}

/// Kafka topics configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
pub struct KafkaTopics {
    /// Topic for raw data ingestion
    pub raw_data: String,

    /// Topic for stream metadata
    pub stream_metadata: String,
}

/// Redis configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
pub struct RedisConfig {
    /// Redis URL
    pub url: String,

    /// Connection pool size
    #[builder(default = default_pool_size())]
    pub pool_size: u32,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_client_id() -> String {
    "aevum-ingestion".to_string()
}

fn default_pool_size() -> u32 {
    10
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::builder().build()
    }
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

        // Add environment variables prefixed with AEVUM_INGESTION_
        config_builder = config_builder.add_source(
            Environment::with_prefix("AEVUM_INGESTION")
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
