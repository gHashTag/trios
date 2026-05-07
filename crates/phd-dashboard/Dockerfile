# R1: Rust-only, no .py/.sh
FROM rust:1.85-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build -p phd-dashboard --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/phd-dashboard /usr/local/bin/phd-dashboard
COPY --from=builder /app/crates/phd-dashboard/templates /templates
ENV RUST_LOG=info
EXPOSE 3030
CMD ["phd-dashboard"]
