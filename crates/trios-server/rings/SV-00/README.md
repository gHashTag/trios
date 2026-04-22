# SV-00 — Router & Bootstrap

## Purpose
Axum router setup, port binding (9005), and server bootstrap.

## API
- `build_router() -> Router`
- `serve(addr: SocketAddr) -> Result<()>`

## Dependencies
- axum, tokio
- SV-01 (MCP handlers)
