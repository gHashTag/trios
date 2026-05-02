# RING — SR-HACK-00 (trios-igla-race-hack)

## Identity

| Field   | Value |
|---------|-------|
| Metal   | 🥉 Bronze |
| Package | `trios-igla-race-hack-sr-hack-00` |
| Sealed  | No |

## Purpose

The vocabulary ring. Everything in GOLD III imports `Term`, `Lane`,
`Gate`, `RingTier` from here. If a downstream ring writes `"pipeline"`
as a free string, the SR-HACK-05 audit will fail.

## Why SR-HACK-00 is the bottom of the graph

SR-HACK-01..05 produce DMs, PR comments, leaderboard rows, Discord
embeds. All of them must reference the same vocabulary. By keeping the
vocabulary in a dependency-free ring, the entire GOLD III branch
compiles in one pass and never accumulates drift.

## API Surface (pub)

| Item              | Role |
|-------------------|------|
| `enum Term`       | 16 canonical concepts (`PipelineO1`, `AlgorithmEntry`, …) |
| `enum Lane`       | Five competitive lanes (`Algorithm`, `TttLora`, `Quantization`, `Megakernels`, `Theory`) |
| `enum Gate`       | Three gate thresholds (`G1`, `G2`, `G3`) |
| `enum RingTier`   | Metal tier (`Gold`, `Silver`, `Bronze`, `ColorVariant(String)`) |
| `fn all_terms()`  | Completeness helper (`Vec<Term>`) for audits |

## Dependencies

- `serde` (derive)            — production
- `serde_json` (`dev-dep`)    — tests only

## Laws

- R1 — pure Rust
- L6 — no I/O, no subprocess, no async
- L13 — I-SCOPE: this ring only
- R-RING-FACADE-001 — outer crate `src/lib.rs` re-exports only
- R-RING-DEP-002 — deps limited to `serde + serde_json`

## Anchor

`φ² + φ⁻² = 3`
