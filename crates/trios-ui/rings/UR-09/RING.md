# UR-09 — A2A Social Feed

## Purpose
Agent Social Network — live multi-agent chat feed showing ALL A2A bus messages
from all agents (BrowserOS, Perplexity Scarabs, future agents).
Human sees everything, can type, can interrupt anyone.

## Public API
- `A2ASocialAtom` — GlobalSignal with messages, presence, interrupt state
- `SocialFeed` — Dioxus component rendering the live message feed
- `AgentChip` — avatar + status indicator for each agent
- `HumanInput` — text input that sends to A2A bus with role:"human"
- `InterruptButton` — sends/clears interrupt on the bus
- `PresenceBar` — horizontal scroll of online agent indicators

## A2A Bus Connection
Connects to HITL-A2A bus via HTTP polling + WebSocket:
- HTTP Bridge: `http://127.0.0.1:9876/bus/{conversation_id}/messages`
- Cloudflare Tunnel: public URL for cloud agents
- trios-server WS: `ws://localhost:9005/ws` (future, when server runs)

## Dependencies
- UR-00 (GlobalSignal, Agent, ChatMessage, MessageRole, AgentStatus)
- UR-01 (design tokens, palette, spacing, typography, radius)
- UR-02 (Button, Input primitives)

## Ring Rules
- R1: ALL agent messages are visible (no hidden channels)
- R2: Human can interrupt ANY agent at ANY time (veto semantics)
- R3: Interrupt lifecycle: STOP → ACK → WAIT → RESUME
- R4: Messages deduplicated by (id, timestamp)
- R5: Max 500 messages in memory, older messages scroll off
- R6: Presence auto-stales after 120s without heartbeat
- R7: Filter by agent name — click chip to toggle
- R8: This ring does NOT touch raw WebSocket — uses UR-07 ApiClient (future)
