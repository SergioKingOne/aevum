# Build stage
FROM rust:1.71-slim-bullseye as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty project
WORKDIR /app
RUN USER=root cargo new --bin aevum-api

# Copy over manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/aevum-common/Cargo.toml ./crates/aevum-common/
COPY crates/aevum-api/Cargo.toml ./crates/aevum-api/

# Build dependencies
RUN mkdir -p ./crates/aevum-common/src && \
    echo "//! Dummy file." > ./crates/aevum-common/src/lib.rs && \
    mkdir -p ./crates/aevum-api/src && \
    echo "fn main() {}" > ./crates/aevum-api/src/main.rs && \
    cargo build --release && \
    rm -rf ./crates/aevum-common/src ./crates/aevum-api/src

# Copy actual source code
COPY crates/aevum-common/src ./crates/aevum-common/src
COPY crates/aevum-api/src ./crates/aevum-api/src

# Build the application
RUN cargo build --release --bin aevum-api

# Final stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/aevum-api /usr/local/bin/

# Create a non-root user
RUN useradd -m aevum
USER aevum

# Create config directory
RUN mkdir -p /home/aevum/config
WORKDIR /home/aevum

# Set the entrypoint
ENTRYPOINT ["aevum-api"]