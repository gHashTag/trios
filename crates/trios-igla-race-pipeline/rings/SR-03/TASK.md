# TASK — SR-03 (trios-igla-race-pipeline)

## Status: DONE ✅

Closes #451 · Part of #446

## Completed

- [x] Ring at `rings/SR-03/` with `README.md`, `TASK.md`, `AGENTS.md`, `RING.md`, `Cargo.toml`, `src/lib.rs`, `schema.sql` (I5)
- [x] `EmaPhiBand` with `α_φ = φ⁻³/2 ≈ 0.1180` (INV-8 PROVEN, Theorem 3.1 SAC-1)
- [x] `PHI_BAND_LOW = 0.10`, `PHI_BAND_HIGH = 0.13` constants
- [x] `EmaPhiBand::with_alpha(α)` rejects α outside the φ-band
- [x] `BpbSink` trait — async-via-pinned-future (R-RING-DEP-002, no global tokio runtime in lib)
- [x] `BpbWriter::for_scarab(_)` / `for_job(_)`
- [x] `write_one()` — O(1) per call, stamps row.ema, forwards to sink
- [x] `WriteErr` (`PhiBandOutOfRange`, `Sink`, `JobIdMismatch`)
- [x] Idempotent `schema.sql` for `scarabs`, `bpb_samples`, `heartbeats` (CREATE TABLE IF NOT EXISTS, CREATE INDEX IF NOT EXISTS)
- [x] 13 unit tests — INV-8 boundaries, EMA convergence, JobId mismatch, schema idempotency, table parity
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `Agent: Bit-Bookkeeper` trailer
- [x] L7 experience entry

## Open (handed to next rings)

- [ ] BR-IO bpb-sqlx-sink — concrete `tokio-postgres` / `sqlx` adapter implementing `BpbSink`
- [ ] SR-02 trainer-runner — emits `BpbSampleRow`s into `BpbWriter::write_one`
- [ ] SR-04 gardener — reads `bpb_samples` for ASHA cull decisions
- [ ] Optional: re-export shims in `crates/trios-igla-race/src/{bpb.rs,ema.rs,neon.rs}`

## Next ring

SR-02 trainer-runner (#454) — first consumer of the writer.
