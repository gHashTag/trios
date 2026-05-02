# AGENTS.md — SR-04 (trios-igla-race-pipeline)

## Identity

- Ring: SR-04
- Package: `trios-igla-race-pipeline-sr-04`
- Role: ASHA decision engine — INV-2 warmup guard, architectural floor, rung ladder
- Soul-name: `Gardener General`
- Codename: `LEAD`

## What this ring does

`Gardener` (stateless), `GardenerAction`, `GardenerDecision`, `GardenerSink` trait, `InvariantStatus`, `pre_flight()`. Pure logic + serde + a mockable async sink trait. No I/O.

## Rules (ABSOLUTE)

- R1   — pure Rust
- L6   — async via trait, no global runtime
- L13  — I-SCOPE: only this ring
- INV-2 — `should_prune` MUST return `false` whenever `step ≤ WARMUP_STEPS`
- ARCH-FLOOR — `should_prune` MUST return `false` whenever `bpb ≥ ARCHITECTURAL_FLOOR_BPB`
- R-RING-DEP-002 — deps = `sr-00 + sr-01 + sr-03 + serde + chrono + thiserror`

## You MAY

- ✅ Add new `GardenerAction` variants (with serde tag)
- ✅ Add new INV-N fields to `InvariantStatus` (Option<bool>)
- ✅ Tune `DEFAULT_RUNGS` IF you bump a guard test
- ✅ Add property tests over the rung ladder

## You MAY NOT

- ❌ Add `sqlx` / `tokio-postgres` / `reqwest` to this ring
- ❌ Cull during warmup (INV-2)
- ❌ Cull a healthy seed at the architectural floor without an explicit Plateau decision
- ❌ Reach into SR-02 trainer-runner (lives above SR-04)

## Build

```bash
cargo build  -p trios-igla-race-pipeline-sr-04
cargo clippy -p trios-igla-race-pipeline-sr-04 --all-targets -- -D warnings
cargo test   -p trios-igla-race-pipeline-sr-04
```

## Anchor

`φ² + φ⁻² = 3` · INV-2 PROVEN · INV-8 read-through from SR-03
