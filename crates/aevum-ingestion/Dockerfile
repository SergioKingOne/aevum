# Build stage
FROM rust:1.85.0-slim-bullseye as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty project
WORKDIR /app
RUN USER=root cargo new --bin aevum-ingestion

# Copy over manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/aevum-common/Cargo.toml ./crates/aevum-common/
COPY crates/aevum-ingestion/Cargo.toml ./crates/aevum-ingestion/

# Build dependencies
RUN mkdir -p ./crates/aevum-common/src && \
    echo "//! Dummy file." > ./crates/aevum-common/src/lib.rs && \
    mkdir -p ./crates/aevum-ingestion/src && \
    echo "fn main() {}" > ./crates/aevum-ingestion/src/main.rs && \
    cargo build --release && \
    rm -rf ./crates/aevum-common/src ./crates/aevum-ingestion/src

# Copy actual source code
COPY crates/aevum-common/src ./crates/aevum-common/src
COPY crates/aevum-ingestion/src ./crates/aevum-ingestion/src

# Build the application
RUN cargo build --release --bin aevum-ingestion

# Final stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/aevum-ingestion /usr/local/bin/

# Create a non-root user
RUN useradd -m aevum
USER aevum

# Create config directory
RUN mkdir -p /home/aevum/config
WORKDIR /home/aevum

# Set the entrypoint
ENTRYPOINT ["aevum-ingestion"]