# RING — SR-04 (trios-igla-race-pipeline)

## Identity

| Field   | Value |
|---------|-------|
| Metal   | 🥈 Silver |
| Package | `trios-igla-race-pipeline-sr-04` |
| Sealed  | No |

## Purpose

ASHA pruner — the part of the gardener that decides whether to cull,
plateau-escalate, or promote a scarab given one `(Scarab, BpbSampleRow)`
observation. Stays Silver (no I/O); the concrete Postgres adapter
ships in a future BR-IO ring.

## Why SR-04 sits above SR-03

SR-04 consumes SR-00 (`Scarab`, `BpbSampleRow`, `JobStatus`), SR-01
(`transition`), and SR-03 (`PHI_BAND_LOW`, `PHI_BAND_HIGH`). It never
reaches across to its peers SR-02 or SR-05 — the gardener is a
feedback loop on the write path, not a new compute path.

## API Surface (pub)

| Item | Role |
|---|---|
| `Gardener` | stateless decision engine |
| `DEFAULT_RUNGS` | 4-rung ASHA schedule (1 K, 3 K, 9 K, 27 K) |
| `WARMUP_STEPS` | INV-2 guard (500) |
| `ARCHITECTURAL_FLOOR_BPB` | 2.19 |
| `GardenerAction` | `Noop`, `Cull{rung, observed_bpb}`, `Plateau{band}`, `Promote` |
| `GardenerDecision` | row (`job_id`, `ts`, `action`, `step`, `bpb`) |
| `GardenerSink` | async trait (mirror of SR-03 `BpbSink`) |
| `InvariantStatus` | INV-1..10 read-through struct |
| `GardenerErr` | `JobIdMismatch`, `Fsm(FsmError)` |

## Dependencies

- `trios-igla-race-pipeline-sr-00` (path)
- `trios-igla-race-pipeline-sr-01` (path)
- `trios-igla-race-pipeline-sr-03` (path)
- `serde`, `chrono`, `thiserror`
- `serde_json` (dev only)

## Laws

- R1 — pure Rust
- L6 — async via trait
- L13 — I-SCOPE: this ring only
- INV-2 — warmup guard enforced in `should_prune`
- R-RING-DEP-002 — strict dep list above

## Anchor

`φ² + φ⁻² = 3`
