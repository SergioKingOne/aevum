-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Create schema
CREATE SCHEMA IF NOT EXISTS aevum;

-- Create streams table
CREATE TABLE IF NOT EXISTS streams (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    schema JSONB NOT NULL,
    retention_days INTEGER NOT NULL DEFAULT 30,
    retention_max_records BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create data_points table (hypertable)
CREATE TABLE IF NOT EXISTS data_points (
    id UUID NOT NULL,
    stream_id UUID NOT NULL REFERENCES streams(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL,
    PRIMARY KEY (id, timestamp)
);

-- Create hypertable
SELECT create_hypertable('data_points', 'timestamp', if_not_exists => TRUE);

-- Create index on stream_id for faster lookups
CREATE INDEX IF NOT EXISTS idx_data_points_stream_id ON data_points(stream_id);

-- Create index on payload for JSON queries
CREATE INDEX IF NOT EXISTS idx_data_points_payload ON data_points USING GIN (payload);

-- Create pipelines table
CREATE TABLE IF NOT EXISTS pipelines (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    source_stream_id UUID NOT NULL REFERENCES streams(id) ON DELETE CASCADE,
    destination_stream_id UUID REFERENCES streams(id) ON DELETE SET NULL,
    operations JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on source_stream_id for faster lookups
CREATE INDEX IF NOT EXISTS idx_pipelines_source_stream_id ON pipelines(source_stream_id);

-- Create users table (for future authentication)
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);