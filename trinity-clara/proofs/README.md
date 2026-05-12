# trinity-clara/proofs — Coq Invariants (IGLA-INV-001..005)

> φ² + φ⁻² = 3 | 5 IGLA invariants (INV-1…INV-5) + audited Trinity Coq corpus
> (gHashTag/t27: 218 statements / 162 Qed / 32 Admitted on 2026-05-12)

## L-R14 (RACE LAW)

```
coqc trinity-clara/proofs/*.v = GREEN  →  RACE VALID
coqc trinity-clara/proofs/*.v ≠ GREEN  →  RACE INVALID
```

## Invariant Map

| File | Invariant | Theorem | Trinity source |
|------|-----------|---------|----------------|
| `igla_asha_bound.v`   | INV-2 | `asha_champion_survives`        | Monte Carlo threshold φ²+φ⁻²+φ⁻⁴ |
| `gf16_precision.v`    | INV-3 | `gf16_safe_domain`              | Lucas closure 6:9 bit split |
| `nca_entropy_band.v`  | INV-4 | `nca_entropy_stability`         | A₅/E₈ symmetry → [1.5, 2.8] |
| `lr_phi_optimality.v` | INV-1 | `bpb_decreases_with_real_gradient` | 7-step αφ derivation |
| `lucas_closure_gf16.v`| INV-5 | `lucas_closure_gf16`            | φ²ⁿ + φ⁻²ⁿ ∈ ℤ ∀n |

## Falsification Protocol

```
JUNO (2026-2027): sin²θ₁₂ ≠ 0.30693  →  Trinity falsified
IGLA (Apr 2026):  champion pruned @ threshold=3.5  →  INV-2 falsified
```

Both are Popper-compliant: concrete condition → concrete falsifiable result.

## Compile

```bash
cd trinity-clara/proofs
coqc lucas_closure_gf16.v
coqc gf16_precision.v
coqc nca_entropy_band.v
coqc lr_phi_optimality.v
coqc igla_asha_bound.v
```

All 5 must exit `0`. Then L-R14 = SATISFIED.

## Connection to the Audited Trinity Coq Corpus

```
Audited Trinity Coq corpus  gHashTag/t27/coq + t27/proofs       [audited 2026-05-12]
                            28 .v files
                            218 statements (122 Theorem + 96 Lemma)
                            162 Qed | 32 Admitted | 11 Abort
ML Invariants (5 new)       trinity-clara/proofs/igla_*.v + lr_*.v + lucas_*.v
```

The earlier "84 + 5 = 89 (F₁₁ Fibonacci prime)" framing was retired in the
2026-05-12 R5-honest sweep: the actual t27 corpus is 218 statements
(≫ 84), and the cosmetic Fibonacci-prime equality no longer reflects the
verified state of the repository. The 5 IGLA invariants below remain the
focal scope of this directory.

## Scientific Principle

> If you are in the correct phase space, the correct answer emerges **without tuning**.
> — A₅ characteristic polynomial gives αφ without free parameters (Trinity paper).
> — Coq invariants enforce correct phase space for ASHA hyperparameter search.
> — Same mathematical principle. Not a metaphor.
