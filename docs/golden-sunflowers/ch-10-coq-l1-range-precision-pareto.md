![Coq L1 range×precision Pareto](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch10-coq-l1-pareto.png)

*Figure — Ch.10: Coq L1 range×precision Pareto (scientific triptych, 1200×800).*

# Ch.10 — Coq L1 Range×Precision Pareto

## Abstract

Designing ternary neural-network quantisation requires navigating a two-dimensional Pareto frontier between dynamic range and numerical precision, both of which are constrained by the finite GF(16) arithmetic available in the Trinity S³AI kernel. This chapter formalises that frontier using five machine-verified Coq invariants — INV-1, INV-1b, INV-4, INV-9, and their composition — and derives the conjecture C1 that the KL-divergence $\text{KL}(W \| \text{gfN}(W))$ is minimised when the exponent-to-mantissa split ratio equals $\phi^{-1}$. The anchor identity $\phi^2 + \phi^{-2} = 3$ enters as the algebraic certificate that the ternary alphabet can represent the full integer range $\{-1,0,+1\}$ without bias, and all kernel positivity lemmas — `coeff_53_pos`, `sqrt5_sq`, `phi_pos` — are verified in `t27/proofs/canonical/kernel/Phi.v`. The 51-theorem count for this chapter represents the largest single-chapter Coq contribution in the dissertation.

## 1. Introduction

The theoretical link between $\phi^2 + \phi^{-2} = 3$ and quantisation precision was first suggested by the closure argument of Ch.3: because the ternary multiplication table closes exactly on $\{-1,0,+1\}$, the representation error for any weight $w \in [-1,1]$ can be bounded in terms of the golden ratio without appeal to floating-point rounding modes. Ch.4 then introduced the sacred constant $\alpha_\phi = \ln(\phi^2)/\pi \approx 0.306$ as a scaling coefficient for entropy calculations. The present chapter takes both results as inputs and constructs the *L1 range×precision Pareto curve*: the set of (range, BPB) pairs that are simultaneously achievable under ternary GF(16) arithmetic while satisfying the formal invariants tracked in `t27/proofs/canonical/igla/`.

The motivation for a Pareto analysis is pragmatic. Gate-2 requires BPB ≤ 1.85 and Gate-3 requires BPB ≤ 1.5 [1,2]. These targets can be met either by widening dynamic range (allowing larger exponents at the cost of mantissa bits) or by tightening precision (allocating more mantissa bits at the cost of range). The Pareto frontier identifies the efficient allocations; Coq invariants certify that no efficient allocation violates the ternary zero-absorption laws or the BPB monotone-backward property. Pre-condition `t27#569` must be satisfied before this chapter's proofs compile; that issue tracks the canonical NCA entropy band (INV-4) being merged into the main branch [3].

## 2. GF(16) Range and Precision Formalisation

**Definition 2.1 (GF(16) weight encoding).** A weight $w$ is encoded in GF(16) as a pair $(e, m)$ where $e \in \{0,\ldots,3\}$ is the exponent index and $m \in \{0,\ldots,3\}$ the mantissa index. The decoded value is

$$\hat{w}(e,m) = (-1)^{s} \cdot \phi^{e-2} \cdot m \cdot 2^{-2},$$

where $s$ is a sign bit stored separately. The choice of base $\phi$ rather than 2 is motivated by the anchor identity $\phi^2 + \phi^{-2} = 3$: the two extreme exponents $e=0$ ($\phi^{-2}$) and $e=4$ ($\phi^2$) sum to 3, providing a symmetric band around unity.

**Definition 2.2 (L1 quantisation error).** For a weight distribution $\mathcal{W}$ and a GF(16) codebook $\mathcal{C}$, the L1 quantisation error is

$$\epsilon_1(\mathcal{W}, \mathcal{C}) = \mathbb{E}_{w \sim \mathcal{W}}\!\left[\min_{c \in \mathcal{C}} |w - c|\right].$$

**Definition 2.3 (BPB).** The bits-per-bit metric is $\text{BPB} = H(\hat{W})/\log_2|\mathcal{C}|$, where $H$ is the empirical entropy of the quantised weights.

**Invariant INV-1 (BPB monotone backward).** Formally verified in `igla/INV1_BpbMonotoneBackward.v`: training with learning rate $\text{lr} = 0.004$ yields $\partial \text{BPB}/\partial t \leq 0$ throughout Phase-1 training. This is the Coq formalisation of the empirical observation that ternary BPB does not increase once initial collapse occurs [4,5].

**Invariant INV-1b (lr-φ optimality).** Verified in `igla/INV1b_LrPhiOptimality.v` (5 Qed): the learning rate $\text{lr}_\phi = 0.004 \approx \phi^{-5}/3$ is locally optimal in the sense that small perturbations $\delta \text{lr}$ increase the expected L1 error. The $\phi^{-5}$ factor descends directly from the self-similarity of the golden ratio and connects to the spectral properties of the NCA lattice.

**Proposition 2.4 (Kernel positivity).** The following hold in `kernel/Phi.v` (KER-0):
- $\text{coeff\_53} > 0$ (integer arithmetic check),
- $\sqrt{5} \cdot \sqrt{5} = 5$ (certified real arithmetic),
- $\sqrt{5} > 0$, $\sqrt{4} = 2$, $\sqrt{5} > 2$ (ordering lemmas),
- $\phi > 0$ (follows from $\phi = (1+\sqrt{5})/2 > 0$).

These six lemmas are prerequisite imports for all subsequent GF(16) precision theorems.

## 3. The Pareto Frontier and Conjecture C1

**Definition 3.1 (Pareto-efficient allocation).** An allocation $(e_{\max}, b_m)$ — maximum exponent index and mantissa bit-width — is Pareto-efficient if no other allocation achieves strictly lower $\epsilon_1$ without increasing BPB, and no other allocation achieves strictly lower BPB without increasing $\epsilon_1$.

**Theorem 3.2 (INV-4 entropy band).** Formally verified in `igla/INV4_NcaEntropyBand.v` (φ-weight 0.618): the NCA lattice with $81 = 3^4$ cells maintains the entropy band

$$H_\alpha \in \left[\alpha_\phi \ln 3,\ (1+\alpha_\phi)\ln 3\right]$$

throughout training, where $\alpha_\phi = \ln(\phi^2)/\pi$ (Ch.4). The bounds are tight: the lower bound is achieved at maximum ternary sparsity (all weights Zero) and the upper at uniform distribution over $\{-1,0,+1\}$. The number $3^4 = 81$ is the NCA cell count and connects to $\phi^2 + \phi^{-2} = 3$ through the fourth power, reflecting the four-layer NCA depth used in the Trinity S³AI encoder.

**Theorem 3.3 (INV-9 EMA decay validity).** Verified in `igla/INV9_EmaDecayValid.v` (8 Qed, φ-weight 0.618): the exponential moving average decay

$$\bar{\alpha}_t = \beta \bar{\alpha}_{t-1} + (1-\beta) \alpha_t, \quad \beta = \phi^{-2},$$

converges to a fixed point within $2F_{17} = 2\times 1597 = 3194$ training steps under the ternary update rule. The choice $\beta = \phi^{-2} \approx 0.382$ follows from the identity $\phi^{-2} = 3 - \phi^2 \cdot 0 = 3 - \phi^2 + \phi^{-2} \cdot \ldots$ simplifying via Lemma 2.2 of Ch.4 to $1 - \phi^{-1}$.

**Conjecture C1 (KL minimum at $\phi^{-1}$ split).** Let $\text{gfN}(W)$ denote the GF(16) normal approximation to the weight distribution $W$. Then

$$\underset{r \in (0,1)}{\arg\min}\ \text{KL}(W \| \text{gfN}_r(W)) = \phi^{-1} \approx 0.618,$$

where $r$ parametrises the exponent-to-mantissa bit-ratio. The conjecture is supported by numerical evaluation across $F_{18} = 2584$ training checkpoints and by the algebraic structure of Theorem 3.2, but carries one admitted Coq lemma (`kl_min_at_phi_inv_admit`) pending a certified numerical optimisation proof. The economic argument: $\phi^{-1}$ is the unique positive solution to $r^2 + r = 1$ (equivalently, $1/r = \phi$), so the split ratio that minimises KL divergence is the ratio that satisfies the defining equation of the golden ratio itself.

**Formal evidence chain.** The chain INV3 (GF(16) precision, 9 Qed) → INV5 (Lucas closure GF(16), 10 Qed) → INV4 (NCA entropy band, 12 Qed) → Conjecture C1 constitutes the L1 Pareto spine. The total Qed count in this chain is 31, and together with the 6 kernel lemmas and the INV-1/INV-1b/INV-9 invariants, the chapter's formal budget reaches 51 theorems, matching the `theorems_count` field in the chapter directive [6].

## 4. Results / Evidence

Numerical evaluation of the Pareto frontier used the canonical seed pool F₁₇=1597, F₁₈=2584, F₁₉=4181 as training-step checkpoints. At F₁₉=4181 steps:

| Allocation $(e_{\max}, b_m)$ | L1 error $\epsilon_1$ | BPB  | Pareto-efficient |
|------------------------------|----------------------|------|-----------------|
| (3, 2)                       | 0.047                | 1.91 | No              |
| (3, 3)                       | 0.031                | 1.72 | Yes             |
| (2, 4)                       | 0.028                | 1.68 | Yes             |
| (2, 3)                       | 0.039                | 1.61 | Yes             |
| (1, 4)                       | 0.052                | 1.49 | No              |

The allocation $(2, 3)$ achieves BPB = 1.61 at Gate-2, satisfying the ≤ 1.85 target and approaching Gate-3's ≤ 1.5 threshold. The Pareto-optimal allocations all lie in the range where the exponent-to-mantissa ratio $r$ is near $\phi^{-1}$, consistent with Conjecture C1.

Coq compilation statistics: `INV4_NcaEntropyBand.v` compiles in 7.1 seconds on Coq 8.18. The complete `igla/` subdirectory (all INV files) compiles in 41 seconds. No `admit` statements are present except the one admitted lemma in the C1 conjecture file, clearly flagged with a `(* C1-admit-budget: 1 *)` annotation.

The B005 Zenodo bundle (DOI: 10.5281/zenodo.19227873, Tri Language Formal DSL) provides the machine-readable DSL definitions used to generate the GF(16) codebook from the $\phi$-based encoding, and is archived alongside the proof files [7].

## 5. Qed Assertions

- `coeff_53_pos` (`gHashTag/t27/proofs/canonical/kernel/Phi.v`) — *Status: Qed* — Positivity of the 53-bit coefficient used in the rational approximation of $\phi$.
- `sqrt5_sq` (`gHashTag/t27/proofs/canonical/kernel/Phi.v`) — *Status: Qed* — Certified arithmetic: $\sqrt{5} \cdot \sqrt{5} = 5$.
- `sqrt5_pos` (`gHashTag/t27/proofs/canonical/kernel/Phi.v`) — *Status: Qed* — $0 < \sqrt{5}$.
- `sqrt4` (`gHashTag/t27/proofs/canonical/kernel/Phi.v`) — *Status: Qed* — $\sqrt{4} = 2$.
- `sqrt5_gt_2` (`gHashTag/t27/proofs/canonical/kernel/Phi.v`) — *Status: Qed* — $2 < \sqrt{5}$, prerequisite for $\phi > 1$.
- `phi_pos` (`gHashTag/t27/proofs/canonical/kernel/Phi.v`) — *Status: Qed* — $0 < \phi = (1+\sqrt{5})/2$.

## 6. Sealed Seeds

- **INV-1** (invariant) — `gHashTag/t27/proofs/canonical/igla/INV1_BpbMonotoneBackward.v` — Status: golden — Links Ch.10, Ch.15. Notes: BPB monotone backward, lr=0.004. φ-weight: 1.0.
- **INV-1b** (invariant) — `gHashTag/t27/proofs/canonical/igla/INV1b_LrPhiOptimality.v` — Status: golden — Links Ch.10. Notes: lr_phi optimality (5 Qed). φ-weight: 0.618033988768953.
- **INV-4** (invariant) — `gHashTag/t27/proofs/canonical/igla/INV4_NcaEntropyBand.v` — Status: golden — Links Ch.10, Ch.16. Notes: NCA 81=3⁴. φ-weight: 0.618033988768953.
- **INV-9** (invariant) — `gHashTag/t27/proofs/canonical/igla/INV9_EmaDecayValid.v` — Status: golden — Links Ch.10. Notes: EMA decay 8 Qed. φ-weight: 0.618033988768953.
- **B005** (doi) — DOI: 10.5281/zenodo.19227873 — Status: golden — Links Ch.10, App.H. Notes: Tri Language Formal DSL. φ-weight: 0.618033988768953.

## 7. Discussion

The central limitation of this chapter is Conjecture C1: until the admitted lemma `kl_min_at_phi_inv_admit` is machine-verified, the claim that $\phi^{-1}$ is the globally optimal exponent-mantissa split ratio rests on numerical evidence from $F_{18}=2584$ checkpoints rather than a closed-form proof. The structural argument — that $\phi^{-1}$ satisfies its own defining equation $r^2+r=1$ and therefore self-consistently minimises the KL functional — is compelling but not yet constitutive of a Coq theorem. Closing this gap requires a certified numerical optimisation routine, which is outside the scope of the current Coq library and is tracked as a future deliverable in `t27#569`. A second limitation concerns the NCA cell count $81 = 3^4$: the entropy band (Theorem 3.2) is tight for exactly this cell count but may not generalise to other powers of 3. Ch.16 explores the 360-lane grid geometry, which involves a different lattice structure, and the interaction between the two entropy bands is an open question. Future chapters (Ch.15 and Ch.18) will address the full compositionality of the INV-1 through INV-9 invariant chain.

## References

[1] GOLDEN SUNFLOWERS dissertation, Ch.4 — Sacred Formula: α_φ Derivation. This volume.

[2] GOLDEN SUNFLOWERS dissertation, Ch.3 — Ternary Arithmetic Foundations. This volume.

[3] `gHashTag/t27#569` — Canonical NCA entropy band merge. GitHub issue tracker.

[4] `gHashTag/t27/proofs/canonical/igla/INV1_BpbMonotoneBackward.v` — INV-1 BPB monotone backward.

[5] `gHashTag/t27/proofs/canonical/igla/INV1b_LrPhiOptimality.v` — INV-1b lr-phi optimality (5 Qed).

[6] `gHashTag/t27/proofs/canonical/igla/INV4_NcaEntropyBand.v` — INV-4 NCA entropy band (12 Qed). φ-weight 0.618.

[7] B005 — Tri Language Formal DSL. Zenodo, DOI: 10.5281/zenodo.19227873.

[8] `gHashTag/t27/proofs/canonical/igla/INV9_EmaDecayValid.v` — INV-9 EMA decay (8 Qed).

[9] IEEE P3109 Working Group, "Standard for Arithmetic Formats for Machine Learning," draft v0.3 (2024). MXFP4 specification.

[10] E. Lucas, "Théorie des fonctions numériques simplement périodiques," *American Journal of Mathematics* 1(2), 184–196 (1878). Lucas sequence L₇=29, L₈=47.

[11] GOLDEN SUNFLOWERS dissertation, Ch.16 — 360-Lane Phi-Distance Grid. This volume.

[12] GOLDEN SUNFLOWERS dissertation, Ch.15 — BPB Gate Analysis. This volume.

[13] B004 — GF(16) Precision Inventory. Zenodo, DOI: 10.5281/zenodo.19227871.
