# AGENTS.md — SR-01 (trios-igla-race-pipeline)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: SR-01
- Package: `trios-igla-race-pipeline-sr-01`
- Role: Job FSM + in-process FIFO queue
- Soul-name: `Queue Quartermaster`
- Codename: `LEAD`

## What this ring does

`transition`, `is_valid_transition`, `StrategyQueue`, `FsmError`. No
I/O, no async, no subprocess. The persistent advisory-lock contention
shim lives in a future BR-IO ring.

## Rules (ABSOLUTE)

- R1   — pure Rust
- L6   — no async, no I/O
- L13  — I-SCOPE: only this ring
- R-RING-DEP-002 — deps = `sr-00 + serde + chrono + thiserror`
- FSM is the contract — it MUST stay frozen unless an ADR
  explicitly amends `JobStatus` in SR-00 first.

## You MAY

- ✅ Add a `claim_one_with(predicate)` variant for SR-04 gardener filters
- ✅ Add `into_iter` / `drain` helpers
- ✅ Add tests, including property tests for FSM closure

## You MAY NOT

- ❌ Loosen the FSM (e.g. accept `Done → Running`)
- ❌ Add tokio / sqlx / reqwest
- ❌ Hold persistent state — that's BR-IO's job

## Build

```bash
cargo build  -p trios-igla-race-pipeline-sr-01
cargo clippy -p trios-igla-race-pipeline-sr-01 --all-targets -- -D warnings
cargo test   -p trios-igla-race-pipeline-sr-01
```

## Anchor

`φ² + φ⁻² = 3`
