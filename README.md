# Aevum: Real-Time Data Analytics Platform

Aevum is a scalable, high-performance data analytics platform designed to ingest, process, analyze, and visualize data streams in real-time. Built with Rust, the platform provides exceptional performance, reliability, and developer experience.

## Features

- **Flexible Data Ingestion**: Support for diverse data sources and formats with configurable validation
- **High-Performance Stream Processing**: Real-time data processing with complex event processing capabilities
- **Advanced Analytics**: Statistical analysis, aggregation, and anomaly detection
- **Time-Series Storage**: Optimized storage for temporal data using TimescaleDB
- **Developer-Friendly APIs**: RESTful APIs for data access and management
- **Enterprise-Grade Security**: Fine-grained access control and audit logging

## Architecture

Aevum follows a microservices architecture with these core components:

1. **Ingestion Service**: Validates and ingests data into Kafka streams
2. **Processor Service**: Applies operations and transformations to data streams
3. **Storage Service**: Persists data in TimescaleDB for efficient time-series access
4. **API Service**: Provides RESTful endpoints for data access and visualization

## Getting Started

### Prerequisites

- Docker and Docker Compose
- Rust 1.67+ (for development)
- PostgreSQL client (for connecting to the database)

### Running Locally

1. Clone the repository:

   ```bash
   git clone https://github.com/your-username/aevum.git
   cd aevum
   ```

2. Start the services with Docker Compose:

   ```bash
   docker-compose up -d
   ```

3. The following services will be available:
   - Ingestion API: http://localhost:8081
   - Query API: http://localhost:8082
   - Kafka Console: http://localhost:8080

### Development Setup

1. Install Rust (if not already installed):

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Install additional tools:

   ```bash
   rustup component add rustfmt clippy
   cargo install cargo-watch cargo-audit
   ```

3. Run the services in development mode:

   ```bash
   # Start dependencies (Kafka, Postgres, Redis)
   docker-compose up -d kafka timescaledb redis

   # Run ingestion service with auto-reload
   cargo watch -x "run --bin aevum-ingestion"
   ```

## API Usage

### Creating a Stream

```bash
curl -X POST http://localhost:8081/streams \
  -H "Content-Type: application/json" \
  -d '{
    "name": "temperature_sensors",
    "description": "Temperature readings from IoT sensors",
    "schema": {
      "type": "object",
      "properties": {
        "sensor_id": { "type": "string" },
        "temperature": { "type": "number" },
        "humidity": { "type": "number" },
        "location": { "type": "string" }
      },
      "required": ["sensor_id", "temperature"]
    }
  }'
```

### Ingesting Data

```bash
# Get the stream ID from the create response
STREAM_ID="your-stream-id"

curl -X POST http://localhost:8081/ingest/${STREAM_ID} \
  -H "Content-Type: application/json" \
  -d '{
    "data": {
      "sensor_id": "sensor-001",
      "temperature": 22.5,
      "humidity": 48.2,
      "location": "kitchen"
    }
  }'
```

### Querying Data

```bash
curl -G http://localhost:8082/data/${STREAM_ID} \
  --data-urlencode "start=2023-01-01T00:00:00Z" \
  --data-urlencode "end=2023-01-02T00:00:00Z" \
  --data-urlencode "limit=100"
```

### Aggregating Data

```bash
curl -G http://localhost:8082/data/${STREAM_ID}/aggregate \
  --data-urlencode "start=2023-01-01T00:00:00Z" \
  --data-urlencode "end=2023-01-02T00:00:00Z" \
  --data-urlencode "interval=1 hour" \
  --data-urlencode "function=avg" \
  --data-urlencode "field=temperature"
```

## Project Structure

```
aevum/
├── Cargo.toml                 # Workspace manifest
├── docker-compose.yml         # Local development environment
├── docker/                    # Dockerfiles for each service
├── migrations/                # Database migrations
├── crates/                    # Workspace members
│   ├── aevum-common/          # Shared code, types, and utilities
│   ├── aevum-ingestion/       # Data ingestion service
│   ├── aevum-processor/       # Stream processing service
│   ├── aevum-storage/         # Storage service
│   └── aevum-api/             # REST API service
└── docs/                      # Documentation
```

## Configuration

Each service can be configured using environment variables. See the `docker-compose.yml` file for examples of configuration options.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch: `git checkout -b feature/my-new-feature`
3. Commit your changes: `git commit -am 'Add some feature'`
4. Push to the branch: `git push origin feature/my-new-feature`
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.
