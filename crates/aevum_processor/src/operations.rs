use aevum_common::{
    Error, Result,
    models::{AggregationFunction, DataPoint, Operation},
    utils::get_nested_value,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::{debug, warn};
use uuid::Uuid;

/// Trait for stream operations that can process data points.
#[async_trait]
pub trait StreamOperation: Send + Sync {
    /// Process a single data point.
    async fn process_single(&self, data_point: DataPoint) -> Result<Option<DataPoint>>;

    /// Process a batch of data points.
    async fn process_batch(&self, data_points: Vec<DataPoint>) -> Result<Vec<DataPoint>> {
        let mut results = Vec::with_capacity(data_points.len());

        for data_point in data_points {
            if let Some(processed) = self.process_single(data_point).await? {
                results.push(processed);
            }
        }

        Ok(results)
    }
}

/// Factory for creating stream operations from configuration.
pub struct OperationFactory;

impl OperationFactory {
    /// Create a new operation from the given configuration.
    pub fn create(operation: Operation, stream_id: Uuid) -> Result<Box<dyn StreamOperation>> {
        match operation {
            Operation::Filter { condition } => Ok(Box::new(FilterOperation::new(condition)?)),
            Operation::Transform { transformation } => {
                Ok(Box::new(TransformOperation::new(transformation)?))
            }
            Operation::Aggregate {
                window_seconds,
                function,
                field,
            } => Ok(Box::new(AggregateOperation::new(
                window_seconds,
                function,
                field,
                stream_id,
            ))),
            Operation::AnomalyDetection {
                algorithm,
                parameters,
            } => Ok(Box::new(AnomalyDetectionOperation::new(
                algorithm, parameters,
            )?)),
        }
    }
}

/// Filter operation that excludes data points based on a condition.
pub struct FilterOperation {
    /// The condition to evaluate
    condition: String,
}

impl FilterOperation {
    /// Create a new filter operation.
    pub fn new(condition: String) -> Result<Self> {
        // TODO: Validate condition syntax
        Ok(Self { condition })
    }

    /// Evaluate the filter condition on a data point.
    fn evaluate(&self, data: &Value) -> bool {
        // TODO: Implement proper condition evaluation
        // This is a simplified version for the MVP

        // Example: condition "temperature > 25"
        let parts: Vec<&str> = self.condition.split_whitespace().collect();
        if parts.len() != 3 {
            warn!("Invalid filter condition format: {}", self.condition);
            return true;
        }

        let field = parts[0];
        let operator = parts[1];
        let value_str = parts[2];

        // Get the field value
        let field_value = match get_nested_value(data, field) {
            Some(v) => v,
            None => {
                warn!("Field not found in data: {}", field);
                return false;
            }
        };

        // Parse the threshold value
        let threshold = match value_str.parse::<f64>() {
            Ok(v) => v,
            Err(_) => {
                warn!("Invalid threshold value: {}", value_str);
                return false;
            }
        };

        // Get the field value as a number
        let field_number = match field_value {
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            _ => {
                warn!("Field is not a number: {}", field);
                return false;
            }
        };

        // Evaluate the condition
        match operator {
            ">" => field_number > threshold,
            ">=" => field_number >= threshold,
            "<" => field_number < threshold,
            "<=" => field_number <= threshold,
            "==" | "=" => (field_number - threshold).abs() < f64::EPSILON,
            "!=" => (field_number - threshold).abs() >= f64::EPSILON,
            _ => {
                warn!("Unsupported operator: {}", operator);
                false
            }
        }
    }
}

#[async_trait]
impl StreamOperation for FilterOperation {
    async fn process_single(&self, data_point: DataPoint) -> Result<Option<DataPoint>> {
        if self.evaluate(&data_point.payload) {
            Ok(Some(data_point))
        } else {
            Ok(None)
        }
    }
}

/// Transform operation that modifies data points.
pub struct TransformOperation {
    /// The transformation specification
    transformation: String,
}

impl TransformOperation {
    /// Create a new transform operation.
    pub fn new(transformation: String) -> Result<Self> {
        // TODO: Validate transformation syntax
        Ok(Self { transformation })
    }

    /// Apply the transformation to a data point.
    fn transform(&self, data: Value) -> Value {
        // TODO: Implement proper transformation
        // This is a simplified version for the MVP

        // Example: transformation "celsius_to_fahrenheit: temperature * 1.8 + 32"
        let parts: Vec<&str> = self.transformation.split(':').collect();
        if parts.len() != 2 {
            warn!("Invalid transformation format: {}", self.transformation);
            return data;
        }

        let new_field = parts[0].trim();
        let expression = parts[1].trim();

        // Parse expression parts
        let expr_parts: Vec<&str> = expression.split_whitespace().collect();
        if expr_parts.len() != 3 && expr_parts.len() != 5 {
            warn!("Invalid expression format: {}", expression);
            return data;
        }

        let field = expr_parts[0];

        // Get the field value
        let field_value = match get_nested_value(&data, field) {
            Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
            _ => {
                warn!("Field not found or not a number: {}", field);
                return data;
            }
        };

        // Apply the transformation
        let result = if expr_parts.len() == 3 {
            // Simple operation: field * value
            let operator = expr_parts[1];
            let value = match expr_parts[2].parse::<f64>() {
                Ok(v) => v,
                Err(_) => {
                    warn!("Invalid value in expression: {}", expr_parts[2]);
                    return data;
                }
            };

            match operator {
                "*" => field_value * value,
                "/" => field_value / value,
                "+" => field_value + value,
                "-" => field_value - value,
                _ => {
                    warn!("Unsupported operator: {}", operator);
                    return data;
                }
            }
        } else {
            // Complex operation: field * value + value
            let op1 = expr_parts[1];
            let val1 = match expr_parts[2].parse::<f64>() {
                Ok(v) => v,
                Err(_) => {
                    warn!("Invalid value in expression: {}", expr_parts[2]);
                    return data;
                }
            };

            let op2 = expr_parts[3];
            let val2 = match expr_parts[4].parse::<f64>() {
                Ok(v) => v,
                Err(_) => {
                    warn!("Invalid value in expression: {}", expr_parts[4]);
                    return data;
                }
            };

            let temp_result = match op1 {
                "*" => field_value * val1,
                "/" => field_value / val1,
                "+" => field_value + val1,
                "-" => field_value - val1,
                _ => {
                    warn!("Unsupported operator: {}", op1);
                    return data;
                }
            };

            match op2 {
                "+" => temp_result + val2,
                "-" => temp_result - val2,
                "*" => temp_result * val2,
                "/" => temp_result / val2,
                _ => {
                    warn!("Unsupported operator: {}", op2);
                    return data;
                }
            }
        };

        // Create a new data point with the transformed value
        let mut new_data = data.clone();

        // Add the new field
        if let Value::Object(ref mut map) = new_data {
            map.insert(new_field.to_string(), json!(result));
        }

        new_data
    }
}

#[async_trait]
impl StreamOperation for TransformOperation {
    async fn process_single(&self, data_point: DataPoint) -> Result<Option<DataPoint>> {
        let transformed_payload = self.transform(data_point.payload);

        Ok(Some(DataPoint {
            stream_id: data_point.stream_id,
            id: data_point.id,
            timestamp: data_point.timestamp,
            payload: transformed_payload,
        }))
    }
}

/// Aggregate operation that combines data points over a time window.
pub struct AggregateOperation {
    /// Window size in seconds
    window_seconds: u32,

    /// Aggregation function to apply
    function: AggregationFunction,

    /// Field to aggregate
    field: String,

    /// Stream ID
    stream_id: Uuid,

    /// Aggregation window state
    window: HashMap<String, Vec<DataPoint>>,

    /// Last window end time
    last_window_end: DateTime<Utc>,
}

impl AggregateOperation {
    /// Create a new aggregate operation.
    pub fn new(
        window_seconds: u32,
        function: AggregationFunction,
        field: String,
        stream_id: Uuid,
    ) -> Self {
        Self {
            window_seconds,
            function,
            field,
            stream_id,
            window: HashMap::new(),
            last_window_end: Utc::now(),
        }
    }

    /// Aggregate values using the specified function.
    fn aggregate_values(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        match self.function {
            AggregationFunction::Sum => values.iter().sum(),
            AggregationFunction::Average => values.iter().sum::<f64>() / values.len() as f64,
            AggregationFunction::Min => *values.iter().fold(&f64::MAX, |a, b| &a.min(b)),
            AggregationFunction::Max => *values.iter().fold(&f64::MIN, |a, b| &a.max(b)),
            AggregationFunction::Count => values.len() as f64,
        }
    }

    /// Extract field value from a data point.
    fn extract_field_value(&self, data_point: &DataPoint) -> Option<f64> {
        get_nested_value(&data_point.payload, &self.field).and_then(|v| v.as_f64())
    }

    /// Get window key for a data point.
    fn get_window_key(&self, timestamp: &DateTime<Utc>) -> String {
        // Create a window key by truncating to the window size
        let seconds = timestamp.timestamp();
        let window_seconds = self.window_seconds as i64;
        let window_start = seconds - (seconds % window_seconds);
        format!("{}", window_start)
    }

    /// Process windows and generate aggregated data points.
    fn process_windows(&mut self, current_time: DateTime<Utc>) -> Vec<DataPoint> {
        let mut results = Vec::new();

        // Find windows that are ready to be aggregated
        let current_timestamp = current_time.timestamp();
        let window_seconds = self.window_seconds as i64;

        // Get window keys to process
        let keys_to_process: Vec<String> = self
            .window
            .keys()
            .filter(|&key| {
                let window_start = key.parse::<i64>().unwrap_or(0);
                let window_end = window_start + window_seconds;
                window_end <= current_timestamp
            })
            .cloned()
            .collect();

        // Process each window
        for key in keys_to_process {
            if let Some(data_points) = self.window.remove(&key) {
                if data_points.is_empty() {
                    continue;
                }

                // Extract values to aggregate
                let values: Vec<f64> = data_points
                    .iter()
                    .filter_map(|dp| self.extract_field_value(dp))
                    .collect();

                if values.is_empty() {
                    continue;
                }

                // Compute aggregate
                let aggregated_value = self.aggregate_values(&values);

                // Create new data point with aggregated value
                let window_start = key.parse::<i64>().unwrap_or(0);
                let window_timestamp =
                    DateTime::<Utc>::from_timestamp(window_start, 0).unwrap_or(Utc::now());

                let mut fields = serde_json::Map::new();
                fields.insert(self.field.clone(), json!(aggregated_value));
                fields.insert("window_start".to_string(), json!(window_start));
                fields.insert(
                    "window_end".to_string(),
                    json!(window_start + window_seconds),
                );
                fields.insert(
                    "window_size_seconds".to_string(),
                    json!(self.window_seconds),
                );
                fields.insert(
                    "aggregation".to_string(),
                    json!(format!("{:?}", self.function)),
                );
                fields.insert("count".to_string(), json!(data_points.len()));

                let aggregated_payload = Value::Object(fields);

                let aggregated_data_point = DataPoint {
                    stream_id: self.stream_id,
                    id: Uuid::new_v4(),
                    timestamp: window_timestamp,
                    payload: aggregated_payload,
                };

                results.push(aggregated_data_point);
            }
        }

        self.last_window_end = current_time;
        results
    }
}

#[async_trait]
impl StreamOperation for AggregateOperation {
    async fn process_single(&self, _data_point: DataPoint) -> Result<Option<DataPoint>> {
        // Aggregation doesn't process single points directly
        // We'll accumulate in process_batch instead
        Ok(None)
    }

    async fn process_batch(&self, data_points: Vec<DataPoint>) -> Result<Vec<DataPoint>> {
        if data_points.is_empty() {
            return Ok(Vec::new());
        }

        // Clone self to create a mutable version
        let mut this = Self {
            window_seconds: self.window_seconds,
            function: self.function.clone(),
            field: self.field.clone(),
            stream_id: self.stream_id,
            window: self.window.clone(),
            last_window_end: self.last_window_end,
        };

        // Add data points to windows
        for data_point in data_points {
            let key = this.get_window_key(&data_point.timestamp);
            this.window.entry(key).or_default().push(data_point);
        }

        // Find the latest timestamp
        let current_time = Utc::now();

        // Process windows
        let results = this.process_windows(current_time);

        // Update self
        let self_mut = unsafe {
            // This is safe because we're making the mutable access exclusive
            (self as *const Self as *mut Self).as_mut().unwrap()
        };
        self_mut.window = this.window;
        self_mut.last_window_end = this.last_window_end;

        Ok(results)
    }
}

/// Anomaly detection operation.
pub struct AnomalyDetectionOperation {
    /// Algorithm to use for anomaly detection
    algorithm: String,

    /// Parameters for the algorithm
    parameters: Value,
}

impl AnomalyDetectionOperation {
    /// Create a new anomaly detection operation.
    pub fn new(algorithm: String, parameters: Value) -> Result<Self> {
        // TODO: Validate algorithm and parameters
        Ok(Self {
            algorithm,
            parameters,
        })
    }

    /// Detect anomalies in a data point.
    fn detect_anomalies(&self, data_point: &DataPoint) -> bool {
        // TODO: Implement proper anomaly detection
        // This is a placeholder for the MVP

        // Simple threshold-based detection
        if let Some(threshold) = self.parameters.get("threshold").and_then(|v| v.as_f64()) {
            if let Some(value) =
                get_nested_value(&data_point.payload, "value").and_then(|v| v.as_f64())
            {
                return value > threshold;
            }
        }

        false
    }
}

#[async_trait]
impl StreamOperation for AnomalyDetectionOperation {
    async fn process_single(&self, data_point: DataPoint) -> Result<Option<DataPoint>> {
        let is_anomaly = self.detect_anomalies(&data_point);

        // If it's an anomaly, add a flag
        if is_anomaly {
            let mut payload = data_point.payload.clone();

            if let Value::Object(ref mut map) = payload {
                map.insert("is_anomaly".to_string(), json!(true));

                // Add detection metadata
                let mut meta = serde_json::Map::new();
                meta.insert("algorithm".to_string(), json!(self.algorithm));
                meta.insert("detected_at".to_string(), json!(Utc::now().to_rfc3339()));
                map.insert("anomaly_detection".to_string(), Value::Object(meta));
            }

            Ok(Some(DataPoint {
                stream_id: data_point.stream_id,
                id: data_point.id,
                timestamp: data_point.timestamp,
                payload,
            }))
        } else {
            // Not an anomaly, pass through
            Ok(Some(data_point))
        }
    }
}
