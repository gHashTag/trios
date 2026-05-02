# AGENTS.md — SR-MEM-01 (trios-agent-memory)

## Identity

- Ring: SR-MEM-01
- Package: `trios-agent-memory-sr-mem-01`
- Role: 4-verb KG adapter contract + retry + circuit breaker
- Soul-name: `Bridge Builder`
- Codename: `LEAD`

## What this ring does

`KgAdapter<B>` (with default + custom config), `KgBackend` trait, `RecallPattern`, `AdapterConfig`, `AdapterErr`. No I/O at this tier — `trios_kg::KgClient` is linked only by the sibling BR-IO ring.

## Rules (ABSOLUTE)

- R1   — pure Rust
- L6   — async via the trait, no global runtime
- L13  — I-SCOPE: only this ring
- R-RING-DEP-002 — deps = `sr-mem-00 + serde + serde_json + thiserror + tracing + tokio`. NO `reqwest`, NO `trios-kg`.
- The retry / breaker numbers are pinned by `AdapterConfig::default()` and tested by `config_defaults_match_acceptance_criteria`. Changing them requires a paired test bump.

## You MAY

- ✅ Add new `KgBackend` impls (in BR-IO rings, not here)
- ✅ Add a `recall_by_id` shortcut
- ✅ Tighten retry policy (more attempts, shorter budget)

## You MAY NOT

- ❌ Pull `reqwest` or `trios-kg` into this ring
- ❌ Add a global runtime
- ❌ Drop the `tracing::instrument` annotations
- ❌ Loosen the breaker (e.g. higher threshold) without an ADR

## Build

```bash
cargo build  -p trios-agent-memory-sr-mem-01
cargo clippy -p trios-agent-memory-sr-mem-01 --all-targets -- -D warnings
cargo test   -p trios-agent-memory-sr-mem-01
```

## Anchor

`φ² + φ⁻² = 3`
