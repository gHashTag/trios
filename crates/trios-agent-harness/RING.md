# trios-agent-harness — Ring Isolation

Agent lifecycle backend. Ported from the TS backend `lib/agents/*` (Wave 2).

```
trios-agent-harness/
├── src/lib.rs          ← re-export facade only (L-ARCH-001)
└── rings/
    ├── AH-00/          ← core types: AgentDefinition, history, stream events
    ├── AH-01/          ← adapter catalog: descriptors, defaults, lookup
    ├── AH-02/          ← message queue: bounded per-agent FIFO
    └── AH-03/          ← turn registry: RingBuffer + TurnRegistry state machine
```

Metal: 🥉 Silver (core/domain).

Dep flow (no sibling imports; higher → lower):
```
  AH-01 → AH-00
  AH-02 → (self-contained)
  AH-03 → AH-00
  facade → AH-00, AH-01, AH-02, AH-03  (re-export only)
```

Notes:
- I/O-free by design. Durable persistence and the async streaming/abort layer
  (SSE pump, AbortController, subscribers, sweep timer) live in the runtime/http
  ring; these rings keep buffering + lifecycle logic deterministic & testable.
- Wire form is camelCase to match the Swift/Hono clients.

Tests: AH-00 ×3, AH-01 ×3, AH-02 ×4, AH-03 ×4 = 14.
