# SR-00 — scarab-types

**Soul-name:** `Scarab Smith` · **Codename:** `LEAD` · **Tier:** 🥉 Bronze · **Kingdom:** Rust

> Closes #448 · Part of #446 · Anchor: `φ² + φ⁻² = 3`

## What this ring does

Dependency-free, serde-only typed primitives for the E2E TTT pipeline.
This ring is the **bottom of the GOLD I dependency graph**; every
other GOLD I ring (SR-01 strategy-queue, SR-02 trainer-runner, SR-03
bpb-writer, SR-04 gardener, SR-05 railway-deployer, BR-OUTPUT
IglaRacePipeline) imports its wire format from here.

## Public types

| Type            | Wire-format role |
|-----------------|------------------|
| `JobId`         | UUID v4 newtype identifying one trainer job |
| `WorkerId`      | String newtype (`<machine>:<worker>`) |
| `Seed`          | i64 newtype for the trainer RNG seed |
| `StrategyId`    | UUID v4 newtype for one strategy enqueued by SR-01 |
| `JobStatus`     | enum: `Queued`, `Running`, `Done`, `Pruned`, `Errored` |
| `Heartbeat`     | per-tick liveness from a worker — 1:1 with `heartbeats` table |
| `BpbSampleRow`  | one BPB observation — 1:1 with `bpb_samples` table |
| `Scarab`        | composite trainer record — 1:1 with `scarabs` table |

## SQL parity

The ring carries a doc-comment that pins each Rust field to its SQL
column. Tests `schema_field_parity_scarab/bpb_sample/heartbeat` assert
that the JSON serialisation always emits the exact column names. If
you rename a field, you MUST ship a paired migration in
`crates/trios-igla-race-pipeline/schema/scarabs_v1.sql` (handed to
SR-03 bpb-writer).

## Dependencies

- `serde` (derive)
- `serde_json` (production — `Scarab::config: serde_json::Value`)
- `uuid` (`v4`, `serde`)
- `chrono` (`serde`)

No `tokio`, no `sqlx`, no `reqwest`, no `anyhow`. (R-RING-DEP-002)

## Build & test

```bash
cargo build  -p trios-igla-race-pipeline-sr-00
cargo clippy -p trios-igla-race-pipeline-sr-00 --all-targets -- -D warnings
cargo test   -p trios-igla-race-pipeline-sr-00
```

13 unit tests, < 0.01 s.

## Backward compatibility

Legacy `crates/trios-igla-race/src/{neon.rs,status.rs}` keeps building.
Re-exports of the SR-00 types from those modules are added in a paired
PR (`feat(igla-race): re-export SR-00 scarab types`) and are **not**
required for this ring to land — SR-00 stands on its own.

## Laws

- L1 — no `.sh`
- L3 — clippy clean
- L6 — no I/O, no async, no subprocess
- L13 — I-SCOPE: this ring only
- L14 — `Agent: LEAD` trailer on commits
- R-RING-FACADE-001 — outer `crates/trios-igla-race-pipeline/src/lib.rs` re-exports only
- R-RING-DEP-002 — deps limited to `serde + serde_json + uuid + chrono`
