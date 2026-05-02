# RING — SR-03 (trios-igla-race-pipeline)

## Identity

| Field   | Value |
|---------|-------|
| Metal   | 🥈 Silver |
| Package | `trios-igla-race-pipeline-sr-03` |
| Sealed  | No |

## Purpose

The canonical BPB write path. SR-03 owns the EMA filter
(`α_φ = φ⁻³/2`, INV-8 PROVEN) and the `BpbSink` boundary that turns
in-memory rows into durable Neon storage.

## Why SR-03 sits above SR-00

SR-00 defines `BpbSampleRow`, `Scarab`, `JobId`. SR-03 turns those
into a stamped, EMA-tagged, durably-written stream. Every other GOLD I
ring (SR-02 trainer-runner, SR-04 gardener, BR-OUTPUT) reads what
SR-03 writes; therefore SR-03 must remain Silver-tier (no I/O of its
own) so the concrete adapter (BR-IO) stays mockable.

## API Surface (pub)

| Item | Role |
|---|---|
| `PHI_BAND_ALPHA` | `α_φ = φ⁻³ / 2 ≈ 0.118034` |
| `PHI_BAND_LOW` / `PHI_BAND_HIGH` | INV-8 φ-band `[0.10, 0.13]` |
| `EmaPhiBand` | Banded EMA |
| `BpbSink` | Async trait — pluggable persistence boundary |
| `BpbWriter` | Composes EMA + Sink, bound to one Scarab |
| `WriteErr` | `PhiBandOutOfRange`, `Sink`, `JobIdMismatch` |
| `SCHEMA_SQL` | Idempotent SQL for `scarabs`, `bpb_samples`, `heartbeats` |

## Dependencies

- `trios-igla-race-pipeline-sr-00` (path) — wire format
- `serde`, `serde_json`, `chrono` — re-asserted SR-00 footprint
- `thiserror` — `WriteErr`
- `tokio` — *dev only* (for `#[tokio::test]`)

No `sqlx`, no `tokio-postgres`, no `reqwest`.

## Laws

- R1 — pure Rust
- L6 — async via trait, no global runtime
- L13 — I-SCOPE: this ring only
- INV-8 — α locked in `[0.10, 0.13]`
- R-RING-DEP-002 — strict dep list above

## Anchor

`φ² + φ⁻² = 3` · `α_φ = φ⁻³ / 2`
