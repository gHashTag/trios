# AGENTS.md — SR-02 (trios-igla-race-pipeline)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: SR-02
- Package: `trios-igla-race-pipeline-sr-02`
- Role: trainer-runner — E2E TTT O(1) per-chunk core
- Soul-name: `Race Runner`
- Codename: `LEAD`

## What this ring does

Owns `TrainerRunner` and `RunResult`. Consumes a `Scarab` from SR-01,
runs the TTT inner loop chunk-by-chunk, emits `Heartbeat` events, and
returns a `RunResult` carrying the best BPB observation for SR-03.

## Rules (ABSOLUTE)

- R1  — Pure Rust only
- L13 — I-SCOPE: only this ring
- R-RING-DEP-002 — no deps beyond SR-00, SR-01, serde, thiserror, chrono (+ optional tch)
- R-RING-FACADE-001 — outer crate `src/lib.rs` re-exports only

## You MAY

- ✅ Add `TrainerRunner`, `RunResult`, `TrainStep`, `HeartbeatSink` trait
- ✅ Gate real libtorch behind `tch` feature
- ✅ Add unit tests with stub trainer (no I/O, no network)

## You MAY NOT

- ❌ Import from SR-03..05 or BR-OUTPUT
- ❌ Add network I/O, filesystem writes, subprocess calls outside `tch` feature
- ❌ Add async / tokio unless gated behind a feature and approved

## Build

```bash
cargo build  -p trios-igla-race-pipeline-sr-02
cargo clippy -p trios-igla-race-pipeline-sr-02 --all-targets -- -D warnings
cargo test   -p trios-igla-race-pipeline-sr-02
```

## Anchor

`φ² + φ⁻² = 3`
