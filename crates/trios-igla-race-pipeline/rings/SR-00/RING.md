# RING — SR-00 (trios-igla-race-pipeline)

## Identity

| Field   | Value |
|---------|-------|
| Metal   | 🥉 Bronze |
| Package | `trios-igla-race-pipeline-sr-00` |
| Sealed  | No |

## Purpose

The wire-format ring. Every other ring in GOLD I imports its types
from here. No I/O, no async. If a downstream ring serialises a free
JSON shape that doesn't go through one of these structs, the SR-04
gardener and SR-03 bpb-writer will fail to deserialise it.

## Why SR-00 is the bottom of the graph

Every other ring (SR-01..05, BR-OUTPUT) depends on `Scarab`,
`BpbSampleRow`, `Heartbeat`, `JobStatus`. If SR-00 had any heavy
dependencies, it would force every downstream ring to inherit them.
Keeping it dep-free guarantees that the entire type graph compiles
in one pass and can be embedded in any future crate (including the
legacy `trios-igla-race`, `trios-railway-audit`, and the planned
`trios-trainer-igla` extraction).

## API Surface (pub)

| Item                         | Role |
|------------------------------|------|
| `JobId(Uuid)`                | UUID v4 newtype |
| `WorkerId(String)`           | `<machine>:<worker>` newtype |
| `Seed(i64)`                  | RNG seed newtype |
| `StrategyId(Uuid)`           | strategy_queue PK newtype |
| `JobStatus`                  | snake_case enum (`queued`/`running`/`done`/`pruned`/`errored`) |
| `Heartbeat`                  | per-tick liveness row |
| `BpbSampleRow`               | one BPB observation |
| `Scarab`                     | composite trainer record + `Scarab::queued()` |

## Dependencies

- `serde` (derive)
- `serde_json`
- `uuid` (`v4`, `serde`)
- `chrono` (`serde`)

## Laws

- R1 — pure Rust
- L6 — no I/O, no async, no subprocess
- L13 — I-SCOPE: this ring only
- R-RING-FACADE-001 — outer crate `src/lib.rs` re-exports only
- R-RING-DEP-002 — deps limited to four crates above

## Anchor

`φ² + φ⁻² = 3`
