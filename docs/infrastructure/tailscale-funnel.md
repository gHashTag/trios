# Tailscale Funnel — trios-server Setup

## ONE COMMAND TO START EVERYTHING

```bash
cargo run -p tri-cli -- start
```

That's it! This command:
1. Starts trios-server on port 9005
2. Starts Tailscale Funnel
3. Shows your cloud URL

## Active Endpoints

```
https://playras-macbook-pro-1.tail01804b.ts.net/
```

| Transport | URL |
|-----------|-----|
| REST (local) | `http://localhost:9005/api/chat` |
| WS (local) | `ws://localhost:9005/ws` |
| SSE (local) | `http://localhost:9005/sse` |
| REST (public) | `https://playras-macbook-pro-1.tail01804b.ts.net/api/chat` |
| WS (public) | `wss://playras-macbook-pro-1.tail01804b.ts.net/ws` |
| Status | `GET /api/status` → `{"agents":0,"status":"ok","tools":31}` |
| Health | `GET /health` → `ok` |

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

# Full chat test
curl -sS -X POST https://playras-macbook-pro-1.tail01804b.ts.net/api/chat \
  -H 'content-type: application/json' \
  -d '{"method":"agents/chat","params":{"message":"pong"}}' | jq
```

## Tailnet IP (internal)

```
http://100.66.38.103:9005/api/status
```

Works without Funnel inside the tailnet.

## Extension Settings

In the Trinity sidepanel → ⚙ Settings → **MCP Server**:
- Click **🖥 Local** → connects to `http://localhost:9005`
- Click **🌐 Public** → connects to `https://playras-macbook-pro-1.tail01804b.ts.net`
- Or type any custom URL manually

The WS connection uses the same base URL with `/ws` appended automatically.

## Server Config (.env)

```
ZAI_API=https://api.z.ai/api/anthropic/v1/messages
ZAI_KEY_1=...
ZAI_KEY_2=...
# ... up to ZAI_KEY_6
TRIOS_REQUEST_TIMEOUT_SECS=120   # reqwest client timeout
```
