# AGENTS.md — SR-ALG-00 (trios-algorithm-arena)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: SR-ALG-00
- Package: `trios-algorithm-arena-sr-alg-00`
- Role: AlgorithmSpec metadata (bottom of GOLD II dependency graph)
- Soul-name: `Arena Architect`
- Codename: `LEAD`

## What this ring does

Defines `AlgorithmId`, `EntryHash`, `GoldenState`, `EnvVar`, `EnvValue`, `AlgorithmSpec`. No logic. No I/O. Pure data + serde.

## Rules (ABSOLUTE)

- R1   — pure Rust
- L6   — no async, no I/O, no subprocess, no network
- L13  — I-SCOPE: only this ring
- R-RING-DEP-002 — deps = `serde + uuid + hex` (+ `serde_json` dev)
- R-RING-FACADE-001 — outer crate `src/lib.rs` re-exports only
- **R-L6-PURE-007** — NO `.py` files in this crate. `entry_path` is a *reference* into `parameter-golf/records/...`. Real Python spawn is SR-02 trainer-runner's job.

## You MAY

- ✅ Add new optional fields to `AlgorithmSpec` (with stable JSON keys)
- ✅ Add new `Display` / `From` impls
- ✅ Add tests, especially property tests on hex roundtrip

## You MAY NOT

- ❌ Add a Python file to this crate
- ❌ Add tokio / sqlx / reqwest
- ❌ Change wire format JSON keys without auditing every consumer ring
- ❌ Drop the 32-byte-length guard in `EntryHash` / `GoldenState`

## Build

```bash
cargo build  -p trios-algorithm-arena-sr-alg-00
cargo clippy -p trios-algorithm-arena-sr-alg-00 --all-targets -- -D warnings
cargo test   -p trios-algorithm-arena-sr-alg-00
```

## Anchor

`φ² + φ⁻² = 3`
