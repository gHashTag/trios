# Dockerfile for trios-dwagent on Railway
FROM rust:1.82-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifest and lock
COPY Cargo.toml Cargo.lock* ./

# Create empty src directory for caching
RUN mkdir src && \
    echo "fn main() {}" > src/lib.rs

# Build dependencies
RUN cargo build --release

# Copy actual source
COPY src ./src

# Rebuild with actual source
RUN touch src/main.rs && \
    cargo build --release && \
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
