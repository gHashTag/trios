# AGENTS.md — SR-00 (trios-igla-race-pipeline)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: SR-00
- Package: `trios-igla-race-pipeline-sr-00`
- Role: scarab-types — wire-format primitives (bottom of GOLD I dependency graph)
- Soul-name: `Scarab Smith`
- Codename: `LEAD`

## What this ring does

Defines `JobId`, `WorkerId`, `Seed`, `StrategyId`, `JobStatus`,
`Heartbeat`, `BpbSampleRow`, `Scarab` and the `Scarab::queued()`
builder. No logic. No I/O. Pure data + serde + chrono + uuid.

## Rules (ABSOLUTE)

- R1  — Pure Rust only
- L6  — no async, no I/O, no subprocess, no network, no tokio, no sqlx
- L13 — I-SCOPE: only this ring
- R-RING-DEP-002 — deps limited to `serde + serde_json + uuid + chrono`
- R-RING-FACADE-001 — outer crate `src/lib.rs` re-exports only

## You MAY

- ✅ Add new newtypes for wire-format primitives
- ✅ Add new `JobStatus` variants (non-breaking)
- ✅ Add fields to `Scarab` / `BpbSampleRow` / `Heartbeat` IF you also ship a paired schema migration
- ✅ Add `impl` blocks and builder methods
- ✅ Add tests

## You MAY NOT

- ❌ Import from any sibling ring (SR-01..05, BR-OUTPUT)
- ❌ Add I/O, filesystem, subprocess, network
- ❌ Add async / tokio / sqlx
- ❌ Rename a field without bumping the schema migration
- ❌ Drop a `JobStatus` variant once shipped (forward-compat)

## Build

```bash
cargo build  -p trios-igla-race-pipeline-sr-00
cargo clippy -p trios-igla-race-pipeline-sr-00 --all-targets -- -D warnings
cargo test   -p trios-igla-race-pipeline-sr-00
```

## Anchor

`φ² + φ⁻² = 3`
