# RING — SR-MEM-05

| Field | Value |
|---|---|
| Tier | 🥈 Silver |
| Crate | `trios-agent-memory-sr-mem-05` |
| Path | `crates/trios-agent-memory/rings/SR-MEM-05/` |
| Deps in | `trios-agent-memory-sr-mem-00`, `trios-agent-memory-sr-mem-01`, `serde`, `serde_json`, `chrono`, `thiserror`, `tracing`, `tokio` (runtime primitives only) |
| Deps out (path) | none yet — wired into BR-OUTPUT after this PR merges |
| I/O | none (Silver-tier; concrete sqlx + Zig-FFI adapters live in sibling BR-IO ring) |
| Public verbs | `Bridge::run`, `Bridge::seed_replay` |
| Public traits | `LessonsSource`, `HdcReplaySource`, `HdcSeedSink` |
| Public types | `LessonRow`, `HdcEpisode`, `BridgeStats`, `BridgeErr` |

## Ring contract

- **In:** events from `LessonsSource` (Neon NOTIFY-style) and
  `HdcReplaySource` (Zig-FFI replay buffer).
- **Process:** translate each event into a `Triple` with full
  `Provenance`, then call `KgAdapter::remember_triple` (SR-MEM-01).
- **Out (forward):** triples persisted into KG; `BridgeStats` returned
  on shutdown.
- **Out (reverse):** `seed_replay` pulls KG triples within `lookback`
  back into a `HdcSeedSink` for warm-start of the HDC replay buffer.

## Sibling BR-IO ring

The concrete `sqlx::PgListener` (Neon NOTIFY) and Zig-FFI shim against
`streaming_memory.zig` live in
`crates/trios-agent-memory/rings/BR-IO-MEM-05/` (future PR). They
implement `LessonsSource` and `HdcReplaySource` against the live data
plane.

🌻 `phi^2 + phi^-2 = 3`
