# Agent Orchestration — Phases 1–5 Implementation

**Date**: 2026-04-22
**Repos**: BrowserOS (dev), trios (feat/223-railway-parallel-training)
**Commits**: `e7427a4c`, `4cac1d91`, `68e5311e`, `dc243a64` (BrowserOS dev)

## What was built

Trinity Agent Bridge chat architecture — 5-phase implementation enabling multi-agent orchestration through MCP tools, SSE real-time events, and a sidepanel Chat tab.

### Phase 1: MCP Tool Schemas
- 4 new MCP tools: `agent_list`, `agent_dispatch`, `agent_chat`, `conversation_history`
- Zod input/output schemas with `response.data()` (L22 compliant)
- Registered in `registry.ts` under "Agent Orchestration (4)"

### Phase 2: Agent Registry + Conversation Store
- In-memory `Map<string, RegisteredAgent>` with soul-names, status, endpoint, heartbeat
- In-memory `Map<string, Conversation>` — append-only per L21
- Auto-registration of agents on first dispatch

### Phase 3: SSE Broadcast Bus
- `AgentEventBus` singleton with publish/subscribe pattern
- `GET /agent-events` — SSE stream with replay support (`?since=ISO`)
- `GET /agent-events/history` — JSON event log for debugging
- `agent_dispatch` publishes `agent_dispatched` events
- `agent_chat` publishes `agent_message` events
- In-memory event log (max 1000 events) for late subscriber replay

### Phase 4: Sidepanel Chat Tab
- `useAgentEvents` hook: SSE connection with exponential backoff reconnection
- `AgentChat` component: real-time event stream with color-coded cards
- Route `/#/agents` in sidepanel (HashRouter)
- Lives in sidepanel.html (not SW) to survive MV3 service worker death

### Phase 5: REST API + Bi-directional Chat
- Exported `dispatchTask()`, `sendChatMessage()`, `listAgents()`, `getConversation()` from agent.ts
- `POST /agent-events/dispatch` — dispatch task to agent
- `POST /agent-events/chat` — send message to conversation
- `GET /agent-events/agents` — list registered agents
- `GET /agent-events/conversation/:id` — get conversation history
- AgentChat input enabled with auto-dispatch on first message

## Key decisions

1. **No MCP Sampling**: The MCP server is created per-request (security fix GHSA-345p-7cg4-v4c7), so MCP Sampling (server → client LLM calls) isn't possible. Instead, the server exposes REST endpoints that the sidepanel calls directly.

2. **SSE over WebSocket**: SSE is simpler, works through proxies, and doesn't need the full MCP transport layer. The sidepanel uses `EventSource` which auto-reconnects.

3. **Sidepanel over SW**: The SSE client lives in sidepanel.html, not the service worker. MV3 kills SW after ~30s idle, which would break SSE connections.

4. **Shared state via module scope**: The agent registry and conversation store are module-scoped singletons in `agent.ts`. Both MCP tools and REST endpoints access the same state.

## Architecture flow

```
Sidepanel AgentChat
  ├─ EventSource → GET /agent-events (SSE stream)
  ├─ fetch() → POST /agent-events/dispatch (new task)
  └─ fetch() → POST /agent-events/chat (multi-turn)

Server
  ├─ agent.ts (MCP tools + shared state)
  ├─ agent-bus.ts (AgentEventBus singleton)
  ├─ agent-events.ts (SSE + REST routes)
  └─ server.ts (Hono app with /agent-events route)

MCP Client (Claude/Cursor)
  └─ Calls agent_list, agent_dispatch, agent_chat, conversation_history
```

## Lessons learned

- **L22 (schema-response parity)**: Every tool with `outputSchema` must call `response.data()`. The MCP SDK validates `structuredContent` against the schema.
- **L23 (no cryptic fallbacks)**: NullBrowser's `unavailable()` pattern gives descriptive errors instead of "X is not a function".
- **Bun SSE**: Use `new ReadableStream()` with `TextEncoder` for SSE in Hono. No special streaming library needed.
- **Hono SSE headers**: Must set `Content-Type: text/event-stream`, `Cache-Control: no-cache`, `Connection: keep-alive`, `X-Accel-Buffering: no`.
