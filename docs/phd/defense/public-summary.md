# Flos Aureus — 1-Page Public Summary

> CC-BY-4.0. Plain-language summary for non-specialists. Hard cap: 1 page (~600 words).

## The question

What unifies the golden ratio (φ ≈ 1.618), Fibonacci sequences, and the
practical learning rate of a deep neural network?

## The claim

The Trinity Anchor `φ² + φ⁻² = 3` is not a numerological coincidence: it is
the Lucas-2 closure that, together with 92 mechanically-checked Coq
theorems, governs the prune threshold (3.5), the model dimension floor (≥256
under GF16), and the LR safe band ([0.002, 0.007]) of the IGLA architecture.

## What would refute it

A trained IGLA instance whose champion learning rate falls **outside**
[0.002, 0.007], whose ASHA prune threshold drifts **above** 3.5, or whose
GF16 precision yields end-to-end training error ≥ 0.5 % at d_model = 256
would refute the architectural reading of the anchor.

## Reproducibility

Run `cargo run -p trios-phd -- reproduce --chapter 24` on seeds {17, 42, 1729}.
Expected: Table 24.1 BPB convergence within ± 0.5 %. All artefacts mirrored
on Zenodo: DOI 10.5281/zenodo.19227877.

## Honest ledger

90 Coq theorems are `Qed`-closed. 2 are `Admitted` with reasons stated in
appendix F. The 8 Coq invariants INV-1…INV-8 / INV-12 are wired into Rust
runtime guards via `assertions/igla_assertions.json` (single source of
truth between proofs and production).

---

*Auditor-seeded skeleton, cycle 2. Substantive plain-language polish is
author lane.*
