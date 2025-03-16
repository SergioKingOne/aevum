use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// Generates a new UUID v4.
pub fn generate_id() -> Uuid {
    Uuid::new_v4()
}

/// Gets the current timestamp.
pub fn current_timestamp() -> DateTime<Utc> {
    Utc::now()
}

/// Validates a JSON value against a JSON Schema.
///
/// # Arguments
///
/// * `value` - The JSON value to validate
/// * `schema` - The JSON Schema to validate against
///
/// # Returns
///
/// * `Ok(())` if validation succeeds
/// * `Err` with a description of the validation error
pub fn validate_json(_value: &Value, _schema: &Value) -> Result<(), String> {
    // TODO: Implement JSON Schema validation
    // This is a placeholder for the MVP
    // We would use a JSON Schema validation library
    Ok(())
}

/// Helper function to generate a timestamp n seconds ago.
pub fn seconds_ago(n: i64) -> DateTime<Utc> {
    Utc::now() - chrono::Duration::seconds(n)
}

/// Safely access a nested field in a JSON object.
pub fn get_nested_value<'a>(json: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json;

    for part in parts {
        match current.get(part) {
            Some(value) => current = value,
            None => return None,
        }
    }

    Some(current)
}
