pub mod config;
pub mod consumer;
pub mod operations;
pub mod pipeline;
pub mod processor;

// Re-export error handling
pub use aevum_common::{Error, Result};
