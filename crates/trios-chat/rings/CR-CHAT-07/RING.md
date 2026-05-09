# RING — CR-CHAT-07 (trios-chat)

## Identity

| Field   | Value |
|---------|-------|
| Tier    | 🥈 Silver (Core ring) |
| Package | `trios-chat-cr-chat-07` |
| Path    | `crates/trios-chat/rings/CR-CHAT-07/` |
| Sealed  | No |

## Purpose

Wire-time **anti-correlation** layer for Trinity Secure Chat. Implements
**R-CHAT-10** (timing privacy). Produces deterministic emission
decisions (Real vs. Cover) and quantises real-time inter-envelope gaps
into one of four canonical classes `{1 s, 5 s, 30 s, 5 min}`.

Pure logic: no async, no I/O, no crypto, no randomness.

## Public API

| Item | Role |
|---|---|
| `CANONICAL_GAPS_MS: [u64; 4]` | the four canonical inter-envelope gap classes |
| `Emission { Real, Cover }` | what the scheduler tells the I/O layer to emit next |
| `CoverScheduler::new()` | empty scheduler |
| `CoverScheduler::enqueue_real(&mut self)` | enqueue one real envelope |
| `CoverScheduler::tick(&mut self) -> Emission` | advance one tick, decide emission |
| `CoverScheduler::queue_depth/&ticks` | introspection getters |
| `uniform_gap_ms(u64) -> u64` | quantise a measured gap into a canonical class |

## Dependencies

| Dep | Why |
|---|---|
| `trios-chat-cr-chat-00` | shared types (currently transitive only — pulled in for future error path) |

No serde, no async, no I/O, no randomness crates.

## Invariants

- `R-CHAT-10 (i)` — for every real envelope produced the scheduler has
  the option of producing zero or more decoy envelopes sandwiching it.
- `R-CHAT-10 (ii)` — `uniform_gap_ms(t) ∈ CANONICAL_GAPS_MS` for all
  `t : u64`.
- `CoverScheduler` is **deterministic** — same call sequence yields
  same emissions. Verified by `falsifier_observer_cannot_count_real_via_emissions`.

## Tests

4 unit tests:
- `scheduler_emits_cover_when_queue_empty` — `[VERIFIED]`
- `scheduler_emits_real_when_queue_nonempty` — `[VERIFIED]`
- `falsifier_observer_cannot_count_real_via_emissions` — `[VERIFIED]`
- `uniform_gap_quantises_to_canonical_set` — `[VERIFIED]`

## Sibling Bronze

`BR-IO-CHAT-07` (future wave) — async wire-emitter that consumes
`CoverScheduler::tick()` and shovels Real/Cover envelopes onto the
mesh on a fixed cadence.

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · UNLINKABLE`
