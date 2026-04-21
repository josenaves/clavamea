# Build Stage
FROM rust:1.81-bookworm AS builder

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /usr/src/clavamea

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

# Pre-cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/clavamea*

# Copy source and build actual binary
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release

# Runtime Stage
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    libssl3 \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/src/clavamea/target/release/clavamea /app/clavamea

# Copy required assets
COPY locales /app/locales
COPY prompts /app/prompts
COPY migrations /app/migrations

# Create directories for persistent data
RUN mkdir -p /app/data /app/memory

# Default environment variables
ENV DATABASE_URL=sqlite:/app/data/clavamea.db
ENV MEMORY_DIR=/app/memory
ENV LOCALES_DIR=/app/locales

ENTRYPOINT ["/app/clavamea"]
