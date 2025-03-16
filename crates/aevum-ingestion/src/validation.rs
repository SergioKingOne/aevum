use aevum_common::{Error, Result, models::Stream, utils::validate_json};
use serde_json::Value;
use tracing::debug;

/// Validates incoming data against a stream's schema.
pub struct DataValidator;

impl DataValidator {
    /// Validate a single data point against a stream's schema.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to validate
    /// * `stream` - The stream containing the schema to validate against
    ///
    /// # Returns
    ///
    /// * `Ok(())` if validation succeeds
    /// * `Err(Error)` if validation fails
    pub fn validate(data: &Value, stream: &Stream) -> Result<()> {
        debug!("Validating data against schema for stream: {}", stream.name);

        validate_json(data, &stream.schema)
            .map_err(|e| Error::Validation(format!("Data validation failed: {}", e)))
    }

    /// Validate a batch of data points against a stream's schema.
    ///
    /// # Arguments
    ///
    /// * `data_batch` - The data points to validate
    /// * `stream` - The stream containing the schema to validate against
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all data points are valid
    /// * `Err(Error)` with the first validation error encountered
    pub fn validate_batch(data_batch: &[Value], stream: &Stream) -> Result<()> {
        debug!(
            "Validating batch of {} data points against schema for stream: {}",
            data_batch.len(),
            stream.name
        );

        for (index, data) in data_batch.iter().enumerate() {
            if let Err(e) = Self::validate(data, stream) {
                return Err(Error::Validation(format!(
                    "Validation failed for item {} in batch: {}",
                    index, e
                )));
            }
        }

        Ok(())
    }
}

/// Validates stream configurations.
pub struct StreamValidator;

impl StreamValidator {
    /// Validate a stream configuration.
    ///
    /// # Arguments
    ///
    /// * `stream` - The stream configuration to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` if validation succeeds
    /// * `Err(Error)` if validation fails
    pub fn validate(stream: &Stream) -> Result<()> {
        debug!("Validating stream configuration: {}", stream.name);

        // Validate basic fields
        if stream.name.trim().is_empty() {
            return Err(Error::Validation("Stream name cannot be empty".to_string()));
        }

        // Validate schema
        if !stream.schema.is_object() {
            return Err(Error::Validation(
                "Stream schema must be a JSON object".to_string(),
            ));
        }

        // Validate retention config
        if stream.retention.days == 0 {
            return Err(Error::Validation(
                "Retention days must be greater than 0".to_string(),
            ));
        }

        // Additional validation can be added here

        Ok(())
    }
}
