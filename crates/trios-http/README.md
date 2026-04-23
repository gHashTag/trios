# trios-http

HTTP REST gateway crate for trios — exposes MCP/WS core over HTTP.

## Architecture

```
Internet / Tailscale Funnel
        ↓
  [trios-http]     ← this crate
        ↓
  [trios-server]   ← WS/MCP core
        ↓
  [trios-core]     ← business logic
```

## Rings

| Ring | Prefix | Responsibility |
|------|--------|----------------|
| HR-00 | core | AppState, ChatRequest, StatusResponse |
| HR-01 | routes | POST /api/chat, GET /api/status, GET /health |
| BR-OUTPUT | binary | axum Router builder, server entrypoint |

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Liveness check |
| `GET` | `/api/status` | `{"status":"ok","agents":N,"tools":19}` |
| `POST` | `/api/chat` | `{"method":"...","params":{...}}` |
| `WS` | `/ws` | WebSocket MCP (from trios-server) |

## Usage

```bash
cargo run -p trios-http
# or via trios-server which embeds these routes
```
