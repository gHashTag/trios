# RING — SR-ALG-03 (trios-algorithm-arena)

## Identity

| Field   | Value |
|---------|-------|
| Metal   | 🥈 Silver |
| Package | `trios-algorithm-arena-sr-alg-03` |
| Lane    | ⭐ WIN |
| Sealed  | No |

## Purpose

Ships the Rust **spec + verifier** for End-to-End Test-Time Training
(arXiv:2512.23675). Pins constants (target BPB, Fibonacci seeds,
entry path, embargo, Coq theorem id), validates the Python entry
script pre-flight (hash, path, env), and evaluates the 3-seed sweep
result strictly against the baseline.

## Why SR-ALG-03 sits above SR-ALG-00

SR-ALG-00 defines `AlgorithmSpec`. SR-ALG-03 is the **first consumer**
— the canonical e2e-ttt manifest. Every other algorithm ring
(SR-ALG-01 jepa, SR-ALG-02 universal-transformer) follows the same
template.

## API Surface (pub)

| Item | Role |
|---|---|
| `TARGET_VAL_BPB` | 1.07063 |
| `FIBONACCI_SEEDS` | [1597, 2584, 4181] |
| `ENTRY_PATH` | pinned relative path |
| `EMBARGO_DAYS` | 14 |
| `COQ_THEOREM_ID` | "alpha_phi_phi_cubed" |
| `E2eTttEnv` | inner_steps / lr_inner / chunk_len |
| `build_spec` | returns `AlgorithmSpec` with theorem pinned |
| `sha256_bytes` | 32-byte SHA-256 helper |
| `Verifier::preflight` | hash + path + env checks |
| `VerifierOutcome` | Ok / HashMismatch / PathUnderCrates / InvalidEnv |
| `evaluate_gate` | strict `mean < TARGET_VAL_BPB` |
| `GateVerdict` | Win / HonestNotYet |

## Dependencies

- `trios-algorithm-arena-sr-alg-00` (path)
- `serde`, `sha2`, `hex`
- `serde_json` (dev only)

## Laws

- R1 — pure Rust
- L6 — no I/O, no async, no subprocess
- L11 — soul-name claimed
- L13 — I-SCOPE: this ring only
- R-L6-PURE-007 — ENTRY_PATH outside crates/
- R-RING-DEP-002 — strict dep list

## Anchor

`φ² + φ⁻² = 3` · WIN target `val_bpb < 1.07063`
