# AGENTS.md — SR-ALG-03 (trios-algorithm-arena)

## Identity

- Ring: SR-ALG-03 ⭐ WIN lane
- Package: `trios-algorithm-arena-sr-alg-03`
- Role: e2e-ttt spec + verifier (no Python execution)
- Soul-name: `Bit Per Byte Hunter`
- Codename: `LEAD`

## What this ring does

Pins constants + spec builder + preflight verifier + gate evaluator for the End-to-End Test-Time Training algorithm from arXiv:2512.23675. NEVER spawns Python; that is SR-02's job.

## Rules (ABSOLUTE)

- R1   — pure Rust
- L6   — no I/O, no async, no subprocess (the verifier consumes already-read bytes)
- L11  — soul-name `Bit Per Byte Hunter` MUST be claimed before any future `train_gpt.py` invocation in SR-02
- L13  — I-SCOPE: only this ring
- R-L6-PURE-007 — `ENTRY_PATH` MUST stay outside `crates/`. The verifier rejects any spec whose `entry_path` starts with or contains `crates/`.
- R-RING-DEP-002 — deps = `sr-alg-00 + serde + sha2 + hex` (+ `serde_json` dev)
- **Strict gate** — `evaluate_gate` uses strict inequality `mean < TARGET_VAL_BPB`. Equality is `HonestNotYet`.

## You MAY

- ✅ Add new env-sanity rules to `Verifier::preflight`
- ✅ Add new `VerifierOutcome` variants (with serde tag)
- ✅ Tighten env defaults IF the paper publishes new ones (bump tests)
- ✅ Add property tests for the gate

## You MAY NOT

- ❌ Touch `train_gpt.py`
- ❌ Copy any `.py` into this crate (R-L6-PURE-007)
- ❌ Loosen the gate to `mean ≤ TARGET_VAL_BPB`
- ❌ Bypass the hash check in `preflight`

## Build

```bash
cargo build  -p trios-algorithm-arena-sr-alg-03
cargo clippy -p trios-algorithm-arena-sr-alg-03 --all-targets -- -D warnings
cargo test   -p trios-algorithm-arena-sr-alg-03
```

## Anchor

`φ² + φ⁻² = 3` · `α_φ = φ⁻³ / 2`
