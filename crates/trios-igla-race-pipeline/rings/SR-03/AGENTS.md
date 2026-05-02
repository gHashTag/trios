# AGENTS.md — SR-03 (trios-igla-race-pipeline)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: SR-03
- Package: `trios-igla-race-pipeline-sr-03`
- Role: BPB write path — EMA(α_φ) + Sink trait + idempotent schema
- Soul-name: `Bit Bookkeeper`
- Codename: `LEAD`

## What this ring does

`EmaPhiBand`, `BpbSink` trait, `BpbWriter`, `WriteErr`, idempotent `SCHEMA_SQL`.

## Rules (ABSOLUTE)

- R1   — Pure Rust
- L6   — async via the trait (no global runtime, no `#[tokio::main]`)
- L13  — I-SCOPE: only this ring
- INV-8 — EMA decay locked at `α_φ = φ⁻³ / 2`. Constructor MUST reject α outside `[0.10, 0.13]`.
- R-RING-DEP-002 — deps = `sr-00 + serde + serde_json + chrono + thiserror`

## You MAY

- ✅ Add new sink-trait implementors (in BR-IO rings, not here)
- ✅ Add a streaming `write_batch` method later (still O(1) amortised)
- ✅ Tighten the φ-band tolerance (NEVER widen it without an ADR)

## You MAY NOT

- ❌ Hardcode an sqlx / tokio-postgres / reqwest dep in this ring
- ❌ Use any α outside `[0.10, 0.13]` on the production path
- ❌ Drop the JobId mismatch guard
- ❌ Render a non-idempotent `CREATE TABLE` in `schema.sql`

## Build

```bash
cargo build  -p trios-igla-race-pipeline-sr-03
cargo clippy -p trios-igla-race-pipeline-sr-03 --all-targets -- -D warnings
cargo test   -p trios-igla-race-pipeline-sr-03
```

## Anchor

`φ² + φ⁻² = 3` · `α_φ = φ⁻³ / 2`
