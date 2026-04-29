# MCP Setup Guide: Connecting Two MCP Servers to Perplexity (Local + Remote)

> **Status**: Local MCP (Mac app) — available now · Remote MCP — coming soon
> **Audience**: Developers and agents working on IGLA RACE / Trinity swarm
> **Platform**: macOS (Mac App Store), Perplexity Pro account required

---

## What is MCP in Perplexity context?

MCP (Model Context Protocol) lets Perplexity connect to external tools, APIs, and services. Think of it as a universal adapter: each MCP server exposes **tools** that Perplexity can call during a conversation.

Two types:
| Type | Where it runs | Auth | Status |
|------|--------------|------|--------|
| **Local MCP** | Your Mac (stdio) | none / local | Available now |
| **Remote MCP** | Cloud (HTTP/SSE) | Bearer token | Coming soon |

---

## SETUP: Two MCP Servers in Perplexity (Mac App)

### Prerequisites

```bash
# 1. Install Node.js (required for npx commands)
brew install node

# 2. Install Perplexity Mac App from Mac App Store
# https://apps.apple.com/app/perplexity-ask-anything/id1668000334

# 3. Perplexity Pro subscription required
```

### Step 1 — Install PerplexityXPC Helper

```
Perplexity App -> Settings -> Connectors -> Install Helper (PerplexityXPC)
```

This helper allows Perplexity to securely communicate with local MCP servers on your machine.

### Step 2 — Add First MCP Server (GitHub / trios)

```
Settings -> Connectors -> Add Connector -> Simple tab
```

| Field | Value |
|-------|-------|
| **Server Name** | `trios-github` |
| **Command** | `npx -y @modelcontextprotocol/server-github` |
| **Env var** | `GITHUB_TOKEN=ghp_your_token_here` |

Or via **Advanced JSON** tab:

```json
{
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-github"],
  "env": {
    "GITHUB_TOKEN": "ghp_your_token_here"
  }
}
```

### Step 3 — Add Second MCP Server (Linear / trios-railway)

```
Settings -> Connectors -> Add Connector -> Simple tab
```

| Field | Value |
|-------|-------|
| **Server Name** | `trios-linear` |
| **Command** | `npx -y @linear/mcp-server` |
| **Env var** | `LINEAR_API_KEY=lin_api_your_key_here` |

Or via **Advanced JSON** tab:

```json
{
  "command": "npx",
  "args": ["-y", "@linear/mcp-server"],
  "env": {
    "LINEAR_API_KEY": "lin_api_your_key_here"
  }
}
```

### Step 4 — Verify Both Servers Are Running

```
Settings -> Connectors -> check "Running" status (green dot)
```

Both servers must show **"Running"** before use.

### Step 5 — Enable in Chat

```
Perplexity homepage -> under "Sources" -> toggle ON each connector
```

---

## REMOTE MCP Setup (coming soon — for trios-mcp server)

When Remote MCP launches, `trios-mcp` deployed on Railway will connect directly:

```json
{
  "mcpServers": {
    "trios-mcp": {
      "url": "https://trios-mcp.railway.app/sse",
      "headers": {
        "Authorization": "Bearer YOUR_MCP_AUTH_BEARER_TOKEN"
      }
    }
  }
}
```

This will expose all 13 IGLA RACE tools:
- `mcp.railway.deploy` — deploy trainer service
- `mcp.hunt.*` — seed hunter operations
- `mcp.exp.next` / `mcp.exp.claim` — EXP_ID management
- `mcp.leaderboard.rank` — BPB leaderboard
- `mcp.smoke.race` — Gate-0 smoke run
- `mcp.github.issue` / `mcp.github.pr` — GitHub ops

---

## ALTERNATIVE: Claude Desktop (works today with Remote MCP)

For immediate use with trios-mcp remote server via Claude Desktop:

```json
// ~/Library/Application Support/Claude/claude_desktop_config.json
{
  "mcpServers": {
    "trios-mcp": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/inspector-cli",
        "https://trios-mcp.railway.app/sse"
      ],
      "env": {
        "MCP_AUTH_BEARER": "your_64_char_token"
      }
    },
    "trios-github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_TOKEN": "ghp_your_token_here"
      }
    }
  }
}
```

Restart Claude Desktop after editing — both servers appear in the tool picker.

---

## Cursor / Continue.dev Config

```json
// .cursor/mcp.json (repo root)
{
  "mcpServers": {
    "trios-mcp": {
      "url": "https://trios-mcp.railway.app/sse",
      "headers": { "Authorization": "Bearer YOUR_TOKEN" }
    },
    "trios-github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_TOKEN": "ghp_your_token" }
    }
  }
}
```

---

## Testing Checklist

- [ ] PerplexityXPC helper installed
- [ ] Server 1 (trios-github) shows "Running" in Connectors
- [ ] Server 2 (trios-linear) shows "Running" in Connectors
- [ ] Both toggled ON under "Sources"
- [ ] Test: "Create a GitHub issue in gHashTag/trios"
- [ ] Test: "List my Linear issues"
- [ ] Verify tool calls appear in activity log

---

## Where to Get Tokens

| Service | Token Location | Used For |
|---------|---------------|----------|
| GitHub | https://github.com/settings/tokens — `repo` scope | trios-github MCP |
| Linear | https://linear.app/settings/api — Personal API Key | trios-linear MCP |
| Railway | https://railway.app/account/tokens | trios-mcp server (acc1/acc2/acc3) |
| Perplexity API | https://www.perplexity.ai/settings/api | Perplexity MCP server |

---

## Related Resources

- [Perplexity MCP Help Center](https://www.perplexity.ai/help-center/en/articles/11502712-local-and-remote-mcps-for-perplexity)
- [Official Perplexity MCP Server](https://github.com/perplexityai/modelcontextprotocol)
- [MCP Server Catalog](https://mcp.perplexity.ai/mcp)
- [trios-mcp Architecture — Issue #333](https://github.com/gHashTag/trios/issues/333)
- [IGLA RACE Master Rules — Issue #331](https://github.com/gHashTag/trios/issues/331)

---

> **Security note**: Never commit API tokens to the repo. Use env vars or `.env` files in `.gitignore`. All docs and code in this repo must be written **in English**.

> phi^2 + phi^-2 = 3 · TRINITY · MCP-ONLY
