# trinity-clara/proofs — Coq Invariants (IGLA-INV-001..005)

> φ² + φ⁻² = 3 | 84 + 5 = **89 theorems** (F₁₁ = 89, Fibonacci prime)

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

## Connection to Existing 84 Theorems

```
Axioms (T1–T3)         trinity-clara/proofs/axioms.v         [pending]
Parametrizations (42)  trinity-clara/proofs/parametrizations.v [pending]
ML Invariants (5 new)  trinity-clara/proofs/igla_*.v + lr_*.v + lucas_*.v
                       ─────────────────────────────────────
                       Total: 84 + 5 = 89 (Fibonacci prime F₁₁)
```

## Scientific Principle

> If you are in the correct phase space, the correct answer emerges **without tuning**.
> — A₅ characteristic polynomial gives αφ without free parameters (Trinity paper).
> — Coq invariants enforce correct phase space for ASHA hyperparameter search.
> — Same mathematical principle. Not a metaphor.
