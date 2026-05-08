# Trinity Mesh Node — CPU daemon for Railway
# φ² + φ⁻² = 3
FROM rust:1.82-slim AS builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/trios-mesh         crates/trios-mesh/
COPY crates/trios-mesh-node    crates/trios-mesh-node/

# Build only mesh-node (avoid full workspace compile)
RUN cargo build --release --package trios-mesh-node 2>&1

# ── Runtime image ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/mesh-node /usr/local/bin/mesh-node
EXPOSE 8080
ENTRYPOINT ["mesh-node"]
