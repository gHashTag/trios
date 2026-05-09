# TASK — CR-CHAT-07 (anti-correlation)

## Status: NEW — Wave-6 ring decomposition

Refs trinity-fpga#28, trinity-fpga#37 · part of
`feat/trios-chat-wave6` (Closes #640).

## Done

- [x] Ring scaffold (RING.md / AGENTS.md / TASK.md / Cargo.toml / src/lib.rs / README.md)
- [x] `CANONICAL_GAPS_MS = [1_000, 5_000, 30_000, 300_000]` constant
- [x] `Emission { Real, Cover }` enum
- [x] `CoverScheduler::new() / enqueue_real / tick / queue_depth / ticks`
- [x] `uniform_gap_ms(u64) -> u64`
- [x] 4 unit tests (cover-when-empty, real-when-nonempty, observer-uniformity falsifier, gap quantisation)

## Open

- [ ] `[ASPIRATIONAL]` BR-IO-CHAT-07 — async wire-emitter ring (next wave)
- [ ] `[ASPIRATIONAL]` Loopix-style multi-hop mix ladder integration

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · UNLINKABLE`
