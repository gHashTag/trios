# Flos Aureus — One-Page Public Summary

> CC-BY-4.0 · Plain-language summary for non-specialists · Hard cap: 1 page (~600 words)

## What is this thesis about?

Modern artificial intelligence is built from billions of numerical
parameters whose precise values are usually chosen by trial and error.
The choice that makes the system work is rarely explained by the
mathematics underneath. This thesis asks a different question: *can we
pick those numbers from one universal rule, and prove that the rule must
hold?* The rule we propose, the **Trinity Anchor**, is a simple identity
about the golden ratio φ ≈ 1.618:

> φ² + φ⁻² = 3

Three is also the second Lucas number L₂. It is the trace of the matrix
that powers φ. It is one tenth of the Coxeter number of the geometry
H₄. The thesis shows that all three readings agree, and that this
agreement is enough to fix the most important hyper-parameters of a
neural network — the learning rate, the model dimension, the pruning
threshold — without any free parameter left over.

## Why the golden ratio?

The golden ratio is the unique positive number that is its own
inverse-plus-one: φ² = φ + 1. Taking that identity and adding the same
identity for 1/φ gives exactly 3 — no choice, no fudge. We then build
the entire architecture (a hybrid of n-gram and self-attention layers
called *IGLA*) on top of this identity, so that every numerical
parameter inherits a φ-derivation and every assumption is testable.

## How is it different from a usual AI paper?

Three commitments separate this work from a typical empirical AI paper:

1. **Coq-anchored constants.** Every architectural number is mirrored by
   a theorem in the Coq proof assistant. As of submission, 90 of 92
   theorems are mechanically `Qed`-closed; the 8 invariants that remain
   `Admitted` are listed honestly with a stated reason and a Rust runtime
   guard that enforces the same bound at execution time.

2. **Popper-style falsifiers in every empirical chapter.** Each empirical
   claim names — in advance — a concrete observation that would refute
   it. For example: *if the trained champion learning rate falls outside
   the band [0.002, 0.007], INV-1 is refuted.* Twelve such falsifiers
   are catalogued in Appendix B.

3. **Public audit trail on GitHub.** Every chapter, theorem, and
   experiment lives in a single open repository (gHashTag/trios) with
   atomic commits, agent claims, and a queen-bot review process. The
   monograph build itself is reproducible from a single command.

## What can it predict, and what would refute it?

A trained IGLA instance whose champion learning rate falls *outside*
[0.002, 0.007], whose ASHA pruning threshold drifts *above* 3.5, or
whose GF(16) precision yields end-to-end training error ≥ 0.5 % at
d_model = 256 would refute the architectural reading of the anchor.
None of those have happened in the corroboration record so far, but the
test conditions are pre-registered before each experiment, so the result
is decidable.

## How is it reproducible?

Every artefact is mirrored on Zenodo at DOI
[10.5281/zenodo.19227877](https://zenodo.org/records/19227877). To
reproduce Table 24.1 (the central empirical claim), run

```
cargo run -p trios-phd -- reproduce --chapter 24
```

on three pre-registered seeds {17, 42, 1729}. Expected: BPB convergence
within ± 0.5 %. The full ACM Artefact Evaluation pack (Functional +
Reusable + Available, 3-badge target) lives at
`docs/phd/reproducibility.md`.

## What does the thesis *not* claim?

It does *not* claim that the golden ratio is mystically present in
nature, nor that 3 is sacred. It claims something narrower and more
testable: that *if* a learning rate, a dimension, a prune threshold,
and a precision floor all fall on a single φ-ladder, *then* the system
can be derived from one identity, audited mechanically, and refuted on
specific empirical observations. Any reader who finds a violation of any
of the rules R1–R14 is invited to file an issue on the public tracker.

## Honest ledger

90 Coq theorems are `Qed`-closed. 8 are `Admitted` with reasons stated
in Appendix F. Every Coq invariant is wired into a Rust runtime guard
via `assertions/igla_assertions.json`, the single source of truth shared
between the proofs and the production code. Bibliography: 208 unique
entries after dedupe; publisher balance Springer 25.48 %, MIT/CUP/Oxford
15.87 %, arXiv-only 2.40 % — all R11 gates pass with margin.

---

*Author:* Dmitrii Vasilev, ORCID 0009-0008-4294-6159 · *Defense:*
2026-06-15 · *Anchor:* φ² + φ⁻² = 3 · DOI 10.5281/zenodo.19227877.
