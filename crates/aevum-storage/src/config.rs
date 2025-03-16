use aevum_common::Error;
use bon::builder;
use config::{Config as ConfigLib, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for the Storage Service.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// Database configuration
    pub database: DatabaseConfig,

    /// Kafka configuration
    pub kafka: KafkaConfig,
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

    /// Migration configuration
    #[builder(default = "default_migration_config()")]
    pub migrations: MigrationConfig,
}

/// Database migration configuration.
#[builder]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationConfig {
    /// Whether to run migrations on startup
    #[builder(default = "default_run_migrations()")]
    pub run_migrations: bool,

    /// Path to migration files
    #[builder(default = "default_migration_path()")]
    pub migration_path: String,
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

    /// Topic for processed data consumption
    pub processed_data: String,

    /// Topic for stream metadata
    pub stream_metadata: String,
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

fn default_migration_config() -> MigrationConfig {
    MigrationConfigBuilder::new()
        .build()
        .expect("Default migration config should be valid")
}

fn default_run_migrations() -> bool {
    true
}

fn default_migration_path() -> String {
    "./migrations".to_string()
}

fn default_client_id() -> String {
    "aevum-storage".to_string()
}

fn default_group_id() -> String {
    "aevum-storage-group".to_string()
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

        // Add environment variables prefixed with AEVUM_STORAGE_
        config_builder = config_builder.add_source(
            Environment::with_prefix("AEVUM_STORAGE")
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
