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

# Terminal 2 — funnel (MUST use App Store Tailscale CLI, NOT brew)
/Applications/Tailscale.app/Contents/MacOS/Tailscale funnel --bg 9005
```

## ⚠️ Critical: Two Tailscale Versions on this Machine

| Source | CLI path | Daemon |
|--------|----------|--------|
| **App Store** (correct) | `/Applications/Tailscale.app/Contents/MacOS/Tailscale` | `IPNExtension` PID ~5646 |
| brew (broken) | `/opt/homebrew/bin/tailscale` | `/var/run/tailscaled.socket` (missing) |

**Always use App Store CLI** — brew CLI cannot connect to the App Store daemon (`IPNExtension`).

Add alias to `~/.zshrc`:

```bash
alias tailscale="/Applications/Tailscale.app/Contents/MacOS/Tailscale"
```

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
/Applications/Tailscale.app/Contents/MacOS/Tailscale funnel --https=443 off
```

## Tailnet IP (internal)

```
http://100.66.38.103:9005/api/status
```

Works without Funnel inside the tailnet.
