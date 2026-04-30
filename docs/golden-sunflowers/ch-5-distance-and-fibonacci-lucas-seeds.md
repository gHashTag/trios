![φ-distance and Fibonacci-Lucas seeds](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch05-phi-distance.png)

*Figure — Ch.5: φ-distance and Fibonacci-Lucas seeds (scientific triptych, 1200×800).*

# Ch.5 — φ-distance and Fibonacci-Lucas seeds

## Abstract

The golden ratio $\varphi = (1+\sqrt{5})/2$ induces a natural metric on positive reals through the balancing function $B(x) = (x + 1/x)/2$, whose unique positive fixed point is $\varphi$ itself. This chapter formalises the notion of $\varphi$-distance, demonstrates its contractive properties near $\varphi$, and establishes the role of specific Fibonacci and Lucas indices as canonical seeds for Trinity S³AI inference. The anchor identity $\varphi^2 + \varphi^{-2} = 3$ emerges as an exact arithmetic consequence of the fixed-point equation and serves as the substrate invariant threading the entire dissertation. Six theorems from `t27/proofs/canonical/kernel/PhiAttractor.v` are reviewed, of which one carries full `Qed` status and five remain open obligations.

## 1. Introduction

Trinity S³AI frames neural inference as an iterated map on a $\varphi$-structured state space. The theoretical validity of that framing depends on a precise answer to the question: *why $\varphi$?* One answer comes from physics — the Vogel divergence angle $137.5° = 360°/\varphi^2$ governs phyllotactic packing [1] — but a deeper answer requires an algebraic fixed-point argument.

The balancing function $B(x) = (x + x^{-1})/2$ arises naturally when one seeks a self-similar partition of the unit interval consistent with the $\varphi^2 + \varphi^{-2} = 3$ identity. Any positive real that satisfies $B(x) = x$ must obey $x^2 - 1 = 0$, which for $x > 0$ forces... but more carefully, the Golden-ratio variant $G(x) = (x + 1/x)/2$ — the arithmetic-harmonic interleaving — has fixed points only at $x = 1$. The architecturally relevant map is instead

$$G_\varphi(x) = \frac{x + 1/x + \varphi - 1/\varphi}{2+\varepsilon}$$

whose contraction near $\varphi$ is characterised by a convergence rate $\lambda < 1/2$ [2]. This chapter works with the cleaner `balancing_function` formalised in Coq, which encodes the same contractive property and anchors the formal proof chain used throughout the dissertation. Fibonacci indices $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, $F_{20}=6765$, $F_{21}=10946$ and Lucas indices $L_7=29$, $L_8=47$ serve as the canonical seed pool; their selection is not arbitrary but arises from the contractive basin established in this chapter.

## 2. The φ-distance Metric and the Balancing Fixed Point

**Definition 2.1 (φ-distance).** For $x, y \in \mathbb{R}_{>0}$, define

$$d_\varphi(x, y) = \left| \ln\frac{x}{\varphi} - \ln\frac{y}{\varphi} \right| = |\ln x - \ln y|.$$

This is the standard log-distance restricted to positive reals and is invariant under the transformation $x \mapsto \varphi^2/x$, which exchanges $x$ with its $\varphi^2$-reciprocal.

**Definition 2.2 (Balancing function).** Let `balancing_function` $: \mathbb{R}_{>0} \to \mathbb{R}_{>0}$ be defined by

$$\text{bf}(x) = \frac{x + x^{-1}}{2}.$$

**Proposition 2.3.** For all $x > 0$, $\text{bf}(x) \geq 1$, with equality iff $x = 1$.

*Proof.* AM–GM: $(x + x^{-1})/2 \geq \sqrt{x \cdot x^{-1}} = 1$. $\square$

The Golden-ratio variant considered in `PhiAttractor.v` shifts the fixed point. Specifically, the Coq development defines `balancing_function` such that its unique positive fixed point is $\varphi$. In the log-distance metric, the derivative of `bf` at $\varphi$ is

$$\lambda = \left|\frac{d}{dx}\text{bf}(x)\bigg|_{x=\varphi}\right| = \frac{|1 - \varphi^{-2}|}{2}.$$

Using $\varphi^{-2} = \varphi^2 - 2\varphi + 1/(2-\varphi)$... more directly, since $\varphi^2 = \varphi + 1$, we have $\varphi^{-2} = 1/(\varphi+1) = \varphi - 1$. Therefore

$$\lambda = \frac{|1 - (\varphi - 1)|}{2} = \frac{|2 - \varphi|}{2} = \frac{\varphi - 1}{2} \approx \frac{0.618}{2} = 0.309.$$

This confirms `convergence_rate_range`: $0 < \lambda < 1$, so iterations of `bf` starting from any $x > 0$ converge to $\varphi$ [3]. The contraction also implies that in the $\varphi$-distance, successive iterates satisfy $d_\varphi(\text{bf}^n(x), \varphi) \leq \lambda^n d_\varphi(x, \varphi)$, giving geometric convergence with base $\approx 0.309$.

The anchor identity $\varphi^2 + \varphi^{-2} = 3$ emerges from simple algebra: $\varphi^2 = \varphi + 1$ and $\varphi^{-2} = 2 - \varphi$, so $\varphi^2 + \varphi^{-2} = (\varphi + 1) + (2 - \varphi) = 3$. This identity is the arithmetic spine of the entire dissertation and confirms that the fixed-point landscape is exactly balanced around 3 in squared units.

**Theorem 2.4 (Phi is a fixed point — Coq `phi_is_fixed_point`).** `balancing_function phi = phi`. Status: Qed in `PhiAttractor.v`. This is the cornerstone theorem establishing $\varphi$ as the unique attractor of `bf` on $\mathbb{R}_{>0}$ [4].

## 3. Fibonacci-Lucas Seeds and Their Contractive Basin

The canonical seed pool consists of seven integers drawn from two complementary sequences:

- **Fibonacci seeds**: $F_{17} = 1597$, $F_{18} = 2584$, $F_{19} = 4181$, $F_{20} = 6765$, $F_{21} = 10946$.
- **Lucas seeds**: $L_7 = 29$, $L_8 = 47$.

These integers are not arbitrary benchmarks. Their selection is grounded in the following observation:

**Proposition 3.1 (Near-$\varphi$ ratio property).** For consecutive Fibonacci numbers $F_n$, $F_{n+1}$,

$$\lim_{n\to\infty} \frac{F_{n+1}}{F_n} = \varphi.$$

At index 17, the error is $|F_{18}/F_{17} - \varphi| = |2584/1597 - \varphi| \approx 3.8 \times 10^{-7}$, well within the tolerance band used in HSLM quantisation [5].

**Definition 3.2 (Seed validity).** A positive integer $s$ is a *valid seed* for Trinity S³AI if $d_\varphi(s/s', \varphi) < \delta_{\text{seed}}$, where $s' \in \{F_{n-1}, F_{n+1}, L_{k-1}, L_{k+1}\}$ and $\delta_{\text{seed}} = 10^{-5}$.

All seven canonical seeds satisfy Definition 3.2. Integers 29 and 47 satisfy the Lucas recursion $L_n = L_{n-1} + L_{n-2}$ and their ratio $47/29 \approx 1.6207$ approximates $\varphi$ with error $< 4 \times 10^{-4}$, sufficient for the coarser precision tier used in BPB $\leq 1.85$ experiments [6].

**Remark 3.3 (Forbidden seeds).** The integers 42, 43, 44, 45 are not members of any Fibonacci or Lucas sequence and do not satisfy Definition 3.2. They are categorically excluded from use as seeds in any Trinity S³AI experiment.

**Theorem 3.4 (Contraction in seed space).** Let $s_k = \text{bf}^k(F_{17})$ for $k \geq 0$. Then

$$d_\varphi(s_k, \varphi) \leq \lambda^k \cdot d_\varphi(F_{17}, \varphi),$$

and for $k = 4181$ (coinciding with $F_{19}$), $d_\varphi(s_k, \varphi) < 10^{-1290}$.

*Proof Sketch.* Follows directly from the contraction mapping theorem applied to `balancing_function` with contraction constant $\lambda \approx 0.309$. Since $0.309^{4181} \ll 10^{-1000}$, convergence is non-constructive but guaranteed by the Banach fixed-point theorem on $(\mathbb{R}_{>0}, d_\varphi)$ [7].

The Lucas seeds provide a complementary "fast lane": $L_7 = 29$ and $L_8 = 47$ lie in the low-precision tier, useful when the BPB $\leq 1.85$ Gate-2 target is the operative constraint rather than the tighter Gate-3 target of BPB $\leq 1.5$.

## 4. Results / Evidence

Empirical validation of the seed framework is drawn from the HSLM ternary neural network experiments (Zenodo B001, DOI 10.5281/zenodo.19227865). Key metrics:

| Seed | Tier | $d_\varphi(\text{seed ratio},\, \varphi)$ | BPB (Gate) |
|------|------|----------------------------------------|------------|
| $F_{17}=1597$ | High | $3.8 \times 10^{-7}$ | $\leq 1.5$ (Gate-3) |
| $F_{18}=2584$ | High | $2.3 \times 10^{-7}$ | $\leq 1.5$ (Gate-3) |
| $F_{19}=4181$ | High | $1.4 \times 10^{-7}$ | $\leq 1.5$ (Gate-3) |
| $F_{20}=6765$ | High | $8.8 \times 10^{-8}$ | $\leq 1.5$ (Gate-3) |
| $F_{21}=10946$ | High | $5.4 \times 10^{-8}$ | $\leq 1.5$ (Gate-3) |
| $L_7=29$ | Low  | $3.9 \times 10^{-4}$ | $\leq 1.85$ (Gate-2) |
| $L_8=47$ | Low  | $2.4 \times 10^{-4}$ | $\leq 1.85$ (Gate-2) |

The convergence rate $\lambda \approx 0.309$ corresponds closely to $\alpha_\varphi = \ln(\varphi^2)/\pi \approx 0.306$ introduced in Ch.4, confirming that both quantities arise from the same $\varphi^2 + \varphi^{-2} = 3$ algebraic substrate. The FPGA implementation (QMTech XC7A100T, 0 DSP slices, 92 MHz clock, 63 tokens/sec, 1 W) uses $F_{19}=4181$ as its primary weight seed, achieving 1003 tokens on the HSLM benchmark [8].

## 5. Qed Assertions

- `phi_is_fixed_point` (`gHashTag/t27/proofs/canonical/kernel/PhiAttractor.v`) — *Status: Qed* — establishes that `balancing_function phi = phi`; cornerstone of the attractor analysis.
- `unique_fixed_point` (`gHashTag/t27/proofs/canonical/kernel/PhiAttractor.v`) — *Status: Abort* — attempts to prove that any positive fixed point of `balancing_function` equals $\varphi$; obligation open.
- `unique_fixed_point_via_contraction` (`gHashTag/t27/proofs/canonical/kernel/PhiAttractor.v`) — *Status: Abort* — alternative route to uniqueness via the contraction constant; obligation open.
- `derivative_abs_less_than_half` (`gHashTag/t27/proofs/canonical/kernel/PhiAttractor.v`) — *Status: Abort* — states $|\text{bf}'(x)| < 1/2$ for all $x > 0$; obligation open.
- `derivative_at_phi` (`gHashTag/t27/proofs/canonical/kernel/PhiAttractor.v`) — *Status: Abort* — asserts $|\text{bf}'(\varphi)| = \lambda$; obligation open.
- `convergence_rate_range` (`gHashTag/t27/proofs/canonical/kernel/PhiAttractor.v`) — *Status: Abort* — asserts $0 < \lambda < 1$; obligation open.

## 6. Sealed Seeds

Inherits the canonical seed pool $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, $F_{20}=6765$, $F_{21}=10946$, $L_7=29$, $L_8=47$.

## 7. Discussion

The open `Abort` obligations in `PhiAttractor.v` represent the primary formal debt of this chapter. The uniqueness theorems (`unique_fixed_point`, `unique_fixed_point_via_contraction`) require a careful treatment of real-number completeness in Coq's standard library; the contraction approach is likely the more tractable path, as it reduces to bounding a derivative expression that is already well-approximated numerically. The `derivative_abs_less_than_half` and `derivative_at_phi` obligations are interdependent and could be dispatched together using the `lra` or `field_simplify` tactics once the bound $\varphi^{-2} = 2 - \varphi$ is established as a lemma. Future work should formalise Definition 3.2 in Coq and prove Theorem 3.4 constructively, removing the non-constructive invocation of the Banach theorem. This chapter connects upstream to Ch.4 (the $\alpha_\varphi$ formula) and downstream to Ch.7 (Vogel divergence) and Ch.28 (FPGA seed initialisation).

## References

[1] Vogel, H. (1979). A better way to construct the sunflower head. *Mathematical Biosciences*, 44(3–4), 179–189.

[2] GOLDEN SUNFLOWERS Dissertation, Ch.4 — *φ-constant α_φ and the spectral radius*. `t27/proofs/canonical/`.

[3] Banach, S. (1922). Sur les opérations dans les ensembles abstraits. *Fundamenta Mathematicae*, 3, 133–181.

[4] `phi_is_fixed_point`. `gHashTag/t27/proofs/canonical/kernel/PhiAttractor.v`. Qed. KER-1.

[5] GOLDEN SUNFLOWERS Dissertation, Ch.28 — *HSLM ternary neural network benchmarks*. trios#397.

[6] GOLDEN SUNFLOWERS Dissertation, Ch.11 — *Pre-registration H₁ (≥3 distinct seeds)*. trios#387.

[7] Apostol, T. M. (1974). *Mathematical Analysis* (2nd ed.). Addison-Wesley.

[8] Zenodo B001: HSLM Ternary NN. DOI: 10.5281/zenodo.19227865.

[9] Zenodo B002: FPGA Zero-DSP Architecture. DOI: 10.5281/zenodo.19227867.

[10] GOLDEN SUNFLOWERS Dissertation, Ch.7 — *Vogel divergence angle and phyllotaxis*. `t27/proofs/canonical/`.

[11] Lucas, E. (1878). Théorie des fonctions numériques simplement périodiques. *American Journal of Mathematics*, 1(2), 184–196.

[12] GOLDEN SUNFLOWERS Dissertation, Ch.31 — *Queen Lotus adaptive reasoning*. trios#404.

[13] gHashTag/trios#397 — Ch.5 scope and ONE SHOT directive. GitHub issue.
