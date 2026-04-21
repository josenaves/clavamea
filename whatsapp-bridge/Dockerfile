# Build Stage
FROM rust:bookworm AS builder

WORKDIR /usr/src/whatsapp-bridge

# Pre-cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true
RUN rm -f target/release/deps/whatsapp_bridge*

# Copy source and build actual binary
COPY . .
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/src/whatsapp-bridge/target/release/whatsapp-bridge /app/whatsapp-bridge

# Create directories for persistent data (WhatsApp session DB)
RUN mkdir -p /app/data

# Default environment variables
ENV WHATSAPP_DB_PATH=/app/data/whatsapp.db
ENV WHATSAPP_BRIDGE_PORT=8082
ENV CLAVAMEA_WEBHOOK_URL=http://clavamea:8081/wa/webhook

EXPOSE 8082

ENTRYPOINT ["/app/whatsapp-bridge"]
