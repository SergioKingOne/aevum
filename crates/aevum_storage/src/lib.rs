pub mod config;
pub mod consumer;
pub mod models;
pub mod repository;

// Re-export error handling
pub use aevum_common::{Error, Result};
