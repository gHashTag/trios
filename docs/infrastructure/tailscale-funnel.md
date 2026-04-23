# Tailscale Funnel — trios-server Setup

## ONE COMMAND TO START EVERYTHING

```bash
cargo run -p tri-cli -- start
```

That's it! This command:
1. Starts trios-server on port 9005
2. Starts Tailscale Funnel
3. Shows your cloud URL

## Endpoint

```
https://playras-macbook-pro-1.tail01804b.ts.net/
```

| Method | URL | Response |
|--------|-----|----------|
| `GET`  | `/api/status` | `{"agents":0,"status":"ok","tools":31}` |
| `POST` | `/api/chat`   | `{"method":"agents/chat","params":{"message":"..."}}` |
| `GET`  | `/health`     | `ok` |
| `WS`   | `/ws`         | WebSocket (MCP protocol) |

## tri CLI Commands

```bash
# Start everything (default)
cargo run -p tri-cli -- start

# Start on custom port
cargo run -p tri-cli -- start --port 3000

# Show status
cargo run -p tri-cli -- status

# Stop everything
cargo run -p tri-cli -- stop
```

## Advanced: Separate Commands

### tri-tunnel CLI (funnel only)

```bash
# Show funnel status
cargo run -p tri-tunnel -- status

# Start funnel on port 9005
cargo run -p tri-tunnel -- start

# Stop funnel
cargo run -p tri-tunnel -- stop
```

### trios-server CLI (server only)

```bash
cargo run -p trios-server
```

## ⚠️ Critical: Tailscale Requirement

Must use **App Store Tailscale CLI**, NOT brew:

| Source | CLI path |
|--------|----------|
| **App Store** (correct) | `/Applications/Tailscale.app/Contents/MacOS/Tailscale` |
| brew (broken) | `/opt/homebrew/bin/tailscale` |

**tri-tunnel automatically uses App Store CLI** — no manual configuration needed.

## Test

```bash
# Local
curl http://localhost:9005/api/status

# Cloud (via Funnel)
curl https://playras-macbook-pro-1.tail01804b.ts.net/api/status
```

## Tailnet IP (internal)

```
http://100.66.38.103:9005/api/status
```

Works without Funnel inside the tailnet.
