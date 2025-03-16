use aevum_common::Error;
use bon::builder;
use config::{Config as ConfigLib, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for the Processor Service.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// Kafka configuration for consuming messages
    pub kafka: KafkaConfig,

    /// Processing configuration
    #[builder(default = "default_processing_config()")]
    pub processing: ProcessingConfig,

    /// Optional Redis configuration for state management
    #[builder(default)]
    pub redis: Option<RedisConfig>,
}

/// Kafka configuration.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Comma-separated list of broker addresses
    pub brokers: String,

    /// Client ID for Kafka connection
    #[builder(default = "default_client_id()")]
    pub client_id: String,

    /// Consumer group ID
    #[builder(default = "default_group_id()")]
    pub group_id: String,

    /// Kafka topics configuration
    pub topics: KafkaTopics,
}

/// Kafka topics configuration.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KafkaTopics {
    /// Topic for raw data consumption
    pub raw_data: String,

    /// Topic for processed data output
    pub processed_data: String,

    /// Topic for stream metadata
    pub stream_metadata: String,

    /// Topic for pipeline configurations
    pub pipelines: String,
}

/// Redis configuration.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis URL
    pub url: String,

    /// Connection pool size
    #[builder(default = "default_pool_size()")]
    pub pool_size: u32,
}

/// Processing configuration.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Number of worker threads for processing
    #[builder(default = "default_workers()")]
    pub workers: usize,

    /// Size of the batch for processing
    #[builder(default = "default_batch_size()")]
    pub batch_size: usize,

    /// Processing interval in milliseconds
    #[builder(default = "default_interval_ms()")]
    pub interval_ms: u64,
}

fn default_processing_config() -> ProcessingConfig {
    ProcessingConfigBuilder::new()
        .build()
        .expect("Default processing config should be valid")
}

fn default_client_id() -> String {
    "aevum-processor".to_string()
}

fn default_group_id() -> String {
    "aevum-processor-group".to_string()
}

fn default_pool_size() -> u32 {
    10
}

fn default_workers() -> usize {
    num_cpus::get()
}

fn default_batch_size() -> usize {
    100
}

fn default_interval_ms() -> u64 {
    100
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

        // Add environment variables prefixed with AEVUM_PROCESSOR_
        config_builder = config_builder.add_source(
            Environment::with_prefix("AEVUM_PROCESSOR")
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
