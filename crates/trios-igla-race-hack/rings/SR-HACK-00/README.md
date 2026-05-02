# SR-HACK-00 — Vocabulary Glossary

**Soul-name:** `Vocab Vigilante` · **Codename:** `LEAD` · **Tier:** 🥉 Bronze · **Kingdom:** Structure

> Closes #447 · Part of #446 · Anchor: `φ² + φ⁻² = 3`

## What this ring does

Pins the L0 partnership-plan vocabulary into Rust. Defines:

- `Term`        — canonical concepts (`PipelineO1`, `AlgorithmEntry`, `Lane`, `Gate`, `RingTier`, …)
- `Lane`        — the five competitive lanes (`Algorithm`, `TttLora`, `Quantization`, `Megakernels`, `Theory`)
- `Gate`        — gate thresholds (`G1`, `G2`, `G3`)
- `RingTier`    — metal tier (`Gold`, `Silver`, `Bronze`, `ColorVariant(_)`)
- `all_terms()` — completeness helper for SR-HACK-01..05 audits

## Why

Every outreach artefact (PR comments, DMs, leaderboard rows, Discord
posts) used to drift apart with re-invented free strings. This ring is
the single source of truth — no more `pipeline ≠ algorithm` confusion.

## Build & test

```bash
cargo build  -p trios-igla-race-hack-sr-hack-00
cargo clippy -p trios-igla-race-hack-sr-hack-00 -- -D warnings
cargo test   -p trios-igla-race-hack-sr-hack-00 --no-deps
```

All tests run in < 5 s, zero clippy warnings.

## Dependencies

- `serde` (derive) — production
- `serde_json` — dev only

That is the entire deps surface (R-RING-DEP-002).

## Laws

- L1 — no `.sh`
- L3 — clippy clean
- L6 — no I/O, no async, no subprocess
- L13 — I-SCOPE: this ring only
- L14 — `Agent: LEAD` trailer on commits
- R-RING-FACADE-001 — outer `crates/trios-igla-race-hack/src/lib.rs` re-exports only
