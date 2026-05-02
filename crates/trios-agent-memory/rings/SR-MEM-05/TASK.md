# SR-MEM-05 — TASK

**Closes:** #455 · **Part of:** #446 · **Soul:** Loop-Locksmith · **Tier:** 🥈 Silver

## Goal

Bidirectional bridge between the two existing episodic stores (Neon `lessons`
table + HDC replay buffer) and the KG long-term memory exposed via
SR-MEM-01.

## Acceptance criteria (issue AC ↔ this ring)

- [x] `rings/SR-MEM-05/` with I5 trinity (README, TASK, AGENTS, RING, Cargo, lib).
- [x] Deps: SR-MEM-00, SR-MEM-01, `tokio` runtime primitives, `tracing`, `thiserror`.
- [x] Forward direction: `Bridge::run` drains `LessonsSource` + `HdcReplaySource`
      into the KG via `KgAdapter::remember_triple`.
- [x] Reverse direction: `Bridge::seed_replay(&mut HdcSeedSink, lookback)` pulls
      KG triples back into the supplied warm-start sink.
- [x] Read-only forwarder on `lessons.rs` (L21 immutability) — `LessonsSource`
      trait has no `&mut` write surface.
- [ ] Smoke against Neon dev DB — **deferred to BR-IO adapter PR**
      (sibling ring; needs `NEON_TEST_URL` CI secret).
- [x] PR closes this issue, `Agent: Loop-Locksmith` trailer.

## Honest scope (R5)

Concrete `sqlx::PgListener` and Zig-FFI HDC reader live in a sibling BR-IO
ring (same precedent as `trios_kg::KgClient` for SR-MEM-01). This ring
ships the contract + state machine + reverse `seed_replay`, all tested
against in-memory mocks.

🌻 `phi^2 + phi^-2 = 3`
