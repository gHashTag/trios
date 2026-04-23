# Tailscale Funnel — trios-server Setup

## Endpoint

```
https://playras-macbook-pro-1.tail01804b.ts.net/
```

| Method | URL | Response |
|--------|-----|----------|
| `GET`  | `/api/status` | `{"agents":0,"status":"ok","tools":19}` |
| `POST` | `/api/chat`   | `{"method":"agents/chat","params":{"message":"..."}}` |
| `GET`  | `/health`     | `ok` |
| `WS`   | `/ws`         | WebSocket (MCP protocol) |

## Start Server + Funnel

```bash
# Terminal 1 — server
cargo run -p trios-server

# Terminal 2 — funnel (NEW: use tri-tunnel)
cargo run -p tri-tunnel -- start
```

### tri-tunnel CLI

```bash
# Show funnel status
cargo run -p tri-tunnel -- status

# Start funnel on port 9005 (default)
cargo run -p tri-tunnel -- start

# Start on custom port
cargo run -p tri-tunnel -- start --port 3000

# Stop funnel
cargo run -p tri-tunnel -- stop
```

### Direct Tailscale CLI (legacy)

```bash
# ONLY if tri-tunnel fails
/Applications/Tailscale.app/Contents/MacOS/Tailscale funnel --bg 9005
```

## ⚠️ Critical: Two Tailscale Versions on this Machine

| Source | CLI path | Daemon |
|--------|----------|--------|
| **App Store** (correct) | `/Applications/Tailscale.app/Contents/MacOS/Tailscale` | `IPNExtension` PID ~5646 |
| brew (broken) | `/opt/homebrew/bin/tailscale` | `/var/run/tailscaled.socket` (missing) |

**tri-tunnel automatically uses App Store CLI** — no manual configuration needed.

## Test

```bash
# Status
curl https://playras-macbook-pro-1.tail01804b.ts.net/api/status
# → {"agents":0,"status":"ok","tools":19}

# Chat
curl -X POST https://playras-macbook-pro-1.tail01804b.ts.net/api/chat \
  -H "Content-Type: application/json" \
  -d '{"method":"agents/list","params":{}}'

# Tools list
curl -X POST https://playras-macbook-pro-1.tail01804b.ts.net/api/chat \
  -H "Content-Type: application/json" \
  -d '{"method":"tools/list","params":{}}'
```

## Disable Funnel

```bash
cargo run -p tri-tunnel -- stop
```

## Tailnet IP (internal)

```
http://100.66.38.103:9005/api/status
```

Works without Funnel inside the tailnet.
