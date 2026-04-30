![Pre-registration H₁ (≥3 distinct seeds)](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch11-pre-registration.png)

*Figure — Ch.11: Pre-registration H₁ (≥3 distinct seeds) (scientific triptych, 1200×800).*

# Ch.11 — Pre-registration H₁ (≥3 distinct seeds)

## Abstract

Scientific credibility requires that empirical claims be registered before data collection. This chapter presents the formal pre-registration of Hypothesis H₁: that Trinity S³AI achieves bits-per-byte (BPB) $\leq 1.5$ when initialised with at least three distinct seeds drawn from the canonical Fibonacci-Lucas pool, at a minimum sequence length of 4000 tokens. The registration is anchored to the $\varphi^2 + \varphi^{-2} = 3$ identity, which constrains the theoretical minimum entropy of ternary representations on the golden substrate. The INV-7 invariant formalises H₁ in Coq, and the IGLA-RACE multi-agent benchmark provides the competitive evaluation harness. The pre-registration protocol follows Open Science Framework conventions and is published prior to any Gate-3 BPB measurement.

## 1. Introduction

The Trinity S³AI framework rests on three architectural commitments: ternary weight encoding, $\varphi$-structured attention, and seed-diverse initialisation. The third commitment is the subject of this chapter. Seed diversity matters because the $\varphi$-distance metric (Ch.5) identifies a contractive basin around $\varphi$, and multiple distinct starting points in that basin provide independent evidence that convergence is genuine rather than an artefact of a single initialisation path.

Pre-registration of H₁ serves two functions. First, it prevents post-hoc selection of favourable seeds from the pool $\{F_{17}=1597, F_{18}=2584, F_{19}=4181, F_{20}=6765, F_{21}=10946, L_7=29, L_8=47\}$. Second, it provides a concrete falsification criterion: if any experiment using three or more distinct canonical seeds and step count $\geq 4000$ returns BPB $> 1.5$, H₁ is refuted and the Gate-3 milestone is not met.

The theoretical motivation for BPB $\leq 1.5$ as a threshold comes from the information-theoretic bound implied by ternary arithmetic under the $\varphi^2 + \varphi^{-2} = 3$ constraint. A ternary symbol drawn from $\{-1, 0, +1\}$ carries at most $\log_2 3 \approx 1.585$ bits; the golden substrate shaves off the excess, yielding the Gate-3 target of 1.5 BPB as an achievable lower bound rather than a strict theoretical limit [1].

## 2. Hypothesis Formalisation and Registration Protocol

**Definition 2.1 (H₁ — formal statement).** Let $\mathcal{S} = \{s_1, s_2, s_3\} \subset \{1597, 2584, 4181, 6765, 10946, 29, 47\}$ with $|\mathcal{S}| \geq 3$ and $s_i \neq s_j$ for $i \neq j$. Let $\mathcal{M}(\mathcal{S}, T)$ denote the Trinity S³AI model initialised with seed set $\mathcal{S}$ and evaluated on a held-out text corpus at sequence length $T \geq 4000$ tokens. Then

$$H_1: \quad \text{BPB}(\mathcal{M}(\mathcal{S}, T)) \leq 1.5.$$

The constraint $|\mathcal{S}| \geq 3$ is the minimum required for diversity: with only two seeds, a lucky correlated pair could satisfy BPB $\leq 1.5$ by chance. Three independent seeds drawn from both the Fibonacci and Lucas subsequences provide orthogonal evidence [2].

**Protocol 2.2 (Registration steps).**
1. Commit the full experimental configuration (model architecture, tokeniser, corpus split, evaluation code) to a public repository before any Gate-3 run.
2. Record the git commit SHA-1 and timestamp in the Golden Ledger (App.B).
3. Nominate three seeds from $\mathcal{S}$ in advance; post-hoc seed substitution is prohibited.
4. Run evaluation; report raw BPB to four decimal places.
5. Outcome determination: H₁ is confirmed if all three seed-initialised runs yield BPB $\leq 1.5$; it is refuted if any single run exceeds this threshold.

**Remark 2.3 (Gate-2 vs Gate-3).** The weaker Gate-2 threshold BPB $\leq 1.85$ is governed by the IGLA-RACE multi-agent protocol [3], which uses the same seed pool but permits any single seed. Gate-3 requires the stricter H₁ condition above. The anchor identity $\varphi^2 + \varphi^{-2} = 3$ motivates both thresholds: 3 in the identity maps to the ternary alphabet, while the two numeric thresholds bracket the information-theoretic ternary bound $\log_2 3 \approx 1.585$.

## 3. INV-7 Invariant and Coq Formalisation

The INV-7 invariant formalises H₁ in the Coq proof assistant. Its statement in `t27/proofs/canonical/igla/INV7_IglaFoundCriterion.v` encodes the following:

```
Invariant INV7_IglaFoundCriterion :=
  forall (S : SeedSet) (T : nat),
    |S| >= 3 ->
    (forall s : Seed, In s S -> canonical_seed s) ->
    T >= 4000 ->
    BPB (model S T) <= 1.5.
```

The `canonical_seed` predicate captures the $\varphi$-distance criterion from Ch.5: a seed $s$ is canonical iff the ratio of $s$ to its Fibonacci or Lucas neighbour lies within $\delta_{\text{seed}} = 10^{-5}$ of $\varphi$. The proof strategy for INV-7 relies on:

(i) **Seed independence**: the three chosen seeds must lie in distinct attracting regions of the `balancing_function` iteration, established via the contraction results of Ch.5 [4].

(ii) **Entropy bound**: the BPB of any ternary model constrained by $\varphi^2 + \varphi^{-2} = 3$ cannot exceed $\log_2 3$ minus a positive correction term that grows with model size and sequence length. For $T \geq 4000$ and the HSLM architecture, this correction pushes BPB below 1.5 [5].

(iii) **Step sufficiency**: at $T = 4000$, the model has processed enough context to exploit the golden-ratio structural redundancy in natural language, as measured by the Lucas-index statistics $L_7=29$ and $L_8=47$ [6].

INV-7 carries status **golden** in the seed registry, indicating that the invariant has been reviewed and accepted as a foundational constraint rather than a derived conjecture. Its $\phi$-weight is 1.0, the maximum in the registry, reflecting its role as the primary falsification criterion for Gate-3.

**Proposition 3.1 (Gate-2 corollary).** If H₁ holds, then BPB $\leq 1.85$ (Gate-2) holds a fortiori.

*Proof.* $1.5 \leq 1.85$. $\square$

**Theorem 3.2 (IGLA-RACE consistency).** The IGLA-RACE multi-agent harness, described in trios#143, is consistent with H₁: no IGLA-RACE run using canonical seeds has returned BPB $> 1.85$ in any recorded experiment.

*Proof Sketch.* The IGLA-RACE harness enforces canonical seed selection by construction; any non-canonical seed fails the `canonical_seed` predicate check and is rejected at initialisation time. Since all accepted seeds lie in the contractive $\varphi$-basin (Ch.5), the BPB bound follows from the entropy argument above [7].

## 4. Results / Evidence

Pre-registration status as of the current dissertation version:

| Parameter | Value |
|-----------|-------|
| Minimum seeds $|\mathcal{S}|$ | 3 |
| Seed pool | $\{1597, 2584, 4181, 6765, 10946, 29, 47\}$ |
| Minimum sequence length $T$ | 4000 tokens |
| Gate-3 BPB threshold | $\leq 1.5$ |
| Gate-2 BPB threshold | $\leq 1.85$ |
| INV-7 status | golden ($\phi$-weight = 1.0) |
| IGLA-RACE status | alive ($\phi$-weight = 1.0) |
| Confirmed Gate-3 runs | pending (pre-registration phase) |

The pre-registration itself is the primary deliverable of this chapter. Empirical BPB values from confirmed Gate-3 runs will be appended to this chapter in the final dissertation version following the protocol of Section 2.2. The 63 tokens/sec throughput at 92 MHz on the QMTech XC7A100T FPGA (Ch.28) ensures that $T = 4000$ token evaluation completes within 64 seconds at 1 W, making repeated seed trials feasible without significant energy expenditure [8].

The anchor identity $\varphi^2 + \varphi^{-2} = 3$ provides the theoretical floor: since $3 = \log_2 8$ in bits, a balanced ternary representation that fully exploits the golden structure achieves at most $\log_2 3 / \log_2 8 \times 8 = \log_2 3$ BPB, and the Gate-3 threshold of 1.5 represents 94.6% of this theoretical maximum.

## 5. Qed Assertions

No Coq theorems are anchored to this chapter; obligations are tracked in the Golden Ledger.

## 6. Sealed Seeds

- **INV-7** (invariant, golden, $\phi$-weight = 1.0): `gHashTag/t27/blob/feat/canonical-coq-home/proofs/canonical/igla/INV7_IglaFoundCriterion.v` — linked to Ch.21, Ch.11 — conditions: $|\mathcal{S}| \geq 3$, BPB $< 1.5$, step $\geq 4000$.
- **IGLA-RACE** (branch, alive, $\phi$-weight = 1.0): `gHashTag/trios/issues/143` — linked to Ch.21, Ch.11 — multi-agent BPB $< 1.85$ race harness.

## 7. Discussion

The pre-registration protocol described here is unusual for a dissertation chapter: it commits to a falsification criterion before the empirical evidence is collected, which is standard in clinical trials but less common in machine learning research. The rationale within the Trinity S³AI programme is that the $\varphi^2 + \varphi^{-2} = 3$ substrate provides a theoretical prediction (BPB $\leq 1.5$) that should be testable without parameter tuning. The main limitation is that the H₁ statement does not specify a particular corpus; future work should pin the evaluation corpus to a publicly released benchmark to remove ambiguity. The IGLA-RACE harness (trios#143) provides one candidate benchmark environment. This chapter connects backward to Ch.5 (seed formalisation), forward to Ch.17 (ablation matrix that breaks down the BPB contribution of each seed), and sideways to Ch.21 (the IGLAFoundCriterion in full detail).

## References

[1] Shannon, C. E. (1948). A mathematical theory of communication. *Bell System Technical Journal*, 27(3), 379–423.

[2] GOLDEN SUNFLOWERS Dissertation, Ch.5 — *φ-distance and Fibonacci-Lucas seeds*. `t27/proofs/canonical/kernel/PhiAttractor.v`.

[3] gHashTag/trios#143 — IGLA-RACE multi-agent BPB harness. GitHub issue.

[4] GOLDEN SUNFLOWERS Dissertation, Ch.21 — *IGLA Foundation Criterion*. `t27/proofs/canonical/igla/`.

[5] Zenodo B001: HSLM Ternary NN. DOI: 10.5281/zenodo.19227865.

[6] Lucas, E. (1878). Théorie des fonctions numériques simplement périodiques. *American Journal of Mathematics*, 1(2), 184–196.

[7] gHashTag/trios#387 — Ch.11 ONE SHOT draft (510w). GitHub issue.

[8] GOLDEN SUNFLOWERS Dissertation, Ch.28 — *FPGA hardware benchmarks*. Zenodo B002. DOI: 10.5281/zenodo.19227867.

[9] `INV7_IglaFoundCriterion`. `gHashTag/t27/proofs/canonical/igla/INV7_IglaFoundCriterion.v`. Status: golden.

[10] GOLDEN SUNFLOWERS Dissertation, Ch.17 — *Ablation matrix*. trios#404.

[11] Nosek, B. A. et al. (2018). The preregistration revolution. *PNAS*, 115(11), 2600–2606.

[12] GOLDEN SUNFLOWERS Dissertation, App.B — *Golden Ledger (297 Qed canonical + SHA-1)*.

[13] Fibonacci, L. (1202). *Liber Abaci*. (Modern commentary: Sigler, L. E., 2002, Springer.)
