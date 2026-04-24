# Dockerfile for trios-dwagent on Railway
FROM rust:slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifest and lock
COPY Cargo.toml Cargo.lock* ./

# Copy actual source
COPY src ./src

# Build
RUN cargo build --release && \
    strip /app/target/release/trios-dwagent

# Runtime image
FROM debian:bookworm-slim

# Install curl for downloading DWAgent
RUN apt-get update && apt-get install -y \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/trios-dwagent /app/trios-dwagent

# Make executable
RUN chmod +x /app/trios-dwagent

# Default command
ENTRYPOINT ["/app/trios-dwagent"]
