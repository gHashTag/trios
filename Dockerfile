FROM rust:1.90-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release -p trios-trainer

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates git && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/trios-train /usr/local/bin/
COPY --from=builder /app/data /app/data
COPY --from=builder /app/assertions /app/assertions
ENTRYPOINT ["trios-train"]
