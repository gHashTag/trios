# Agent Instructions — BR-OUTPUT (trios-server)

## Context
Server binary ring. Must always compile. Port 9005 is sacred.

## Verification
```bash
cargo build -p trios-server
cargo run -p trios-server &
curl http://localhost:9005/sse
```
