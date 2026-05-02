# SR-MEM-05 — Episodic Bridge (lessons.rs + HDC ↔ KG)

**Soul-name:** `Loop-Locksmith` · **Codename:** `LEAD` · **Tier:** 🥈 Silver

> Closes #455 · Part of #446 · Anchor: `φ² + φ⁻² = 3`

## Honest scope (R5)

This ring ships the **bridge contract + state machine**, not the concrete
`sqlx::PgListener` and Zig-FFI HDC reader. The trade-off mirrors SR-MEM-01:

- Issue AC asks for `sqlx` (Neon NOTIFY) + Zig FFI (`streaming_memory.zig`).
  Pulling `sqlx` (with `runtime-tokio-rustls`) and a Zig FFI shim into a
  Silver ring would inherit native-TLS + the full Postgres surface and
  violate `R-RING-DEP-002` (no I/O at Silver tier). The issue itself
  acknowledges Zig FFI is not yet ready ("FFI binding marked TODO if
  Zig FFI not yet ready").
- Instead, SR-MEM-05 takes two trait objects:
  - `LessonsSource` — yields `LessonRow` events (concrete sqlx PgListener
    impl ships in a sibling BR-IO ring).
  - `HdcReplaySource` — yields `HdcEpisode` events (concrete Zig-FFI impl
    ships in a sibling BR-IO ring once `streaming_memory.zig` exports
    stabilise).
- Forward direction (`Bridge::run`) and reverse direction
  (`Bridge::seed_replay`) are both implemented for real against the trait
  objects + a `KgBackend` (re-used from SR-MEM-01). Mock backends in
  tests prove every contract clause.

The 4-clause contract from the issue is honoured 1:1:

| Issue AC | Where it lives in this ring |
|---|---|
| Subscribe to Neon NOTIFY → forward as triple via SR-MEM-01 | `Bridge::run` over `LessonsSource::stream()` → `KgAdapter::remember_triple` |
| HDC forwarder reads `streaming_memory.zig` | `Bridge::run` over `HdcReplaySource::stream()` |
| Reverse: `seed_replay(&mut HdcMemory, lookback: Duration)` pulls KG triples back | `Bridge::seed_replay(&mut H: HdcSeedSink, lookback: Duration)` |
| Read-only forwarder on `lessons.rs` (L21 immutability) | trait `LessonsSource::stream` returns `LessonRow` events; no write surface exists |

## API

```rust
pub struct Bridge<L: LessonsSource, H: HdcReplaySource, B: KgBackend>;

impl<L, H, B> Bridge<L, H, B>
where
    L: LessonsSource,
    H: HdcReplaySource,
    B: KgBackend,
{
    pub fn new(lessons: L, hdc: H, adapter: KgAdapter<B>) -> Self;

    /// Forward direction. Drains both sources concurrently into the KG.
    /// Returns once both source streams complete or `cancel.notified()` fires.
    pub async fn run(&self, cancel: tokio::sync::Notify) -> BridgeStats;

    /// Reverse direction. Pulls KG triples within `lookback` back into the
    /// supplied seed sink (e.g. an HDC replay buffer's warm-start path).
    pub async fn seed_replay<S: HdcSeedSink>(
        &self,
        sink: &mut S,
        lookback: Duration,
    ) -> Result<usize, BridgeErr>;
}

pub trait LessonsSource: Send + Sync { /* stream() -> LessonRow */ }
pub trait HdcReplaySource: Send + Sync { /* stream() -> HdcEpisode */ }
pub trait HdcSeedSink: Send + Sync { /* seed(&Triple) -> Result */ }

pub struct LessonRow { kind, subject, predicate, object, ts }
pub struct HdcEpisode { hyper_id, subject, predicate, object, ts }
pub struct BridgeStats { lessons_forwarded, hdc_forwarded, errors }
pub enum  BridgeErr { Source(String), Adapter(AdapterErr), Cancelled }
```

## Tests

| Group | Tests |
|---|---|
| Forward | `forward_lessons_into_kg`, `forward_hdc_into_kg`, `forward_concurrent_both_sources` |
| Reverse | `seed_replay_pulls_recent_triples`, `seed_replay_respects_lookback`, `seed_replay_handles_empty_kg` |
| Cancellation | `run_honors_cancel_notify` |
| Errors | `forward_continues_after_source_error`, `seed_replay_propagates_kg_error` |
| Read-only | `lessons_source_is_immutable_view` (compile-time: trait has no `&mut self`) |
| φ-anchor | `phi_anchor_present` |

All tests use in-memory mocks (`MockLessons`, `MockHdc`, `MockKg`).

## Dependencies (R-RING-DEP-002)

```
serde, serde_json, chrono, thiserror, tracing, tokio
+ trios-agent-memory-sr-mem-00 (Triple, TripleId, Provenance, AgentRole)
+ trios-agent-memory-sr-mem-01 (KgAdapter, KgBackend, RecallPattern)
```

No `sqlx`, no `reqwest`, no Zig FFI. Those land in the sibling BR-IO ring.

## Smoke-test deferral (R5)

The issue AC closes with `Smoke test against Neon dev DB`. The sandbox has
no `NEON_DATABASE_URL` and the concrete listener/FFI adapters live in
BR-IO. The smoke test ships with the BR-IO adapter PR + a `NEON_TEST_URL`
CI secret. This ring's contract tests are the layer that's verifiable
right now.

🌻 `α_φ = φ⁻³ / 2 ≈ 0.1180` · `phi^2 + phi^-2 = 3`
