# RING — SR-ALG-00 (trios-algorithm-arena)

## Identity

| Field   | Value |
|---------|-------|
| Metal   | 🥉 Bronze |
| Package | `trios-algorithm-arena-sr-alg-00` |
| Sealed  | No |

## Purpose

Wire-format ring for the algorithm arena. SR-ALG-01..03 + BR-OUTPUT
all import their manifest types from here. No I/O, no async — pure
data + serde.

## Why SR-ALG-00 is the bottom of the graph

Every other ring in GOLD II (jepa, universal-transformer, e2e-ttt,
arena assembler) depends on `AlgorithmSpec`, `EntryHash`,
`GoldenState`. Keeping SR-ALG-00 dep-free guarantees the entire GOLD
II branch compiles in one pass.

## API Surface (pub)

| Item | Role |
|---|---|
| `AlgorithmId(Uuid)` | unique submission id |
| `EntryHash([u8; 32])` | SHA-256 hex newtype |
| `GoldenState([u8; 32])` | optional convergence-checkpoint hash |
| `EnvVar(String)` / `EnvValue(String)` | env-pair newtypes |
| `AlgorithmSpec` | full manifest with `verify_hash()` |

## Dependencies

- `serde` (derive)
- `uuid` (`v4`, `serde`)
- `hex` — lowercase hex serialisation
- `serde_json` (dev only)

## Laws

- R1 — pure Rust
- L6 — no I/O, no async
- L13 — I-SCOPE: this ring only
- R-RING-DEP-002 — strict dep list above
- R-RING-FACADE-001 — outer crate re-exports only
- R-L6-PURE-007 — no `.py` in this crate

## Anchor

`φ² + φ⁻² = 3`
