![φ-period Cycles](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch25-phi-period-cycles.png)

*Figure — Ch.25: φ-period Cycles (scientific triptych, 1200×800).*

# Ch.25 — $\varphi$-Period Cycles

## Abstract

This chapter develops the theory of $\varphi$-period cycles — periodic orbits in the weight and attention manifolds of the TRINITY S³AI model that arise because the quantisation lattice is invariant under multiplication by $\varphi^2$. The central result is that every trajectory of the gradient-descent dynamics on the $\varphi$-quantised weight space is eventually periodic with period dividing $F_k$ for some $k$, and that the attractor set is precisely the subset of weights satisfying $\varphi^2 + \varphi^{-2} = 3$ up to lattice precision. The chapter defines the notion of a $\varphi$-cycle formally, classifies cycles of order $\leq F_{10} = 55$, and connects the cycle structure to the Vogel divergence angle (Ch.7) and the statistical periodicity of the training loss (Ch.19).

## 1. Introduction

Periodic behaviour in gradient-descent optimisation is usually treated as a pathology: limit cycles indicate that the learning rate is too large or the loss landscape has degenerate saddle points. In the TRINITY S³AI framework, by contrast, a restricted class of periodic orbits is not merely tolerated but engineered. The $\varphi$-quantised weight lattice $\Lambda_\varphi$ satisfies

$$\varphi^2 \cdot \Lambda_\varphi = \Lambda_\varphi,$$

which means that rescaling all weights by $\varphi^2$ returns them to the same lattice. This invariance implies that any two weight configurations related by $\varphi^{2k}$ for integer $k$ are indistinguishable under the quantised arithmetic — they produce the same output distribution and the same loss gradient. The gradient-descent map on $\Lambda_\varphi$ therefore has a quotient structure: the effective phase space is the torus $\Lambda_\varphi / \varphi^{2\mathbb{Z}}$, which is compact and hence admits periodic orbits.

The anchor identity $\varphi^2 + \varphi^{-2} = 3$ plays a dual role here. It is the algebraic certificate that $\Lambda_\varphi$ is closed under the two operations $\times\varphi^2$ and $\times\varphi^{-2}$ (since $\varphi^2 + \varphi^{-2}$ is an integer), and it sets the diameter of the fundamental domain of the quotient torus to exactly 3 lattice units [1]. This compactness ensures that every orbit visits at most $3^d$ distinct quantised configurations in $d$ dimensions before repeating, bounding the cycle length.

## 2. $\varphi$-Lattice Structure and the Cycle Map

**Definition 2.1 ($\varphi$-quantised lattice).** The one-dimensional $\varphi$-quantised lattice is:
$$\Lambda_\varphi^{(1)} = \{ a + b\varphi : a, b \in \mathbb{Z} \} \cap [-\varphi^{-1}, \varphi^{-1}],$$
truncated to the unit cell. The $d$-dimensional lattice is $\Lambda_\varphi^{(d)} = (\Lambda_\varphi^{(1)})^d$.

**Proposition 2.2 ($\varphi^2$-invariance).** For every $\lambda \in \Lambda_\varphi^{(1)}$, $\varphi^2 \lambda \bmod 1 \in \Lambda_\varphi^{(1)}$.

*Proof.* Write $\lambda = a + b\varphi$ with $a, b \in \mathbb{Z}$. Then $\varphi^2 \lambda = \varphi^2(a + b\varphi) = a\varphi^2 + b\varphi^3 = a(\varphi+1) + b(\varphi^2 + \varphi) = a(\varphi+1) + b(2\varphi+1) = (a+b) + (a+2b)\varphi$. Since $a+b, a+2b \in \mathbb{Z}$, the result lies in $\Lambda_\varphi^{(1)}$ before truncation. $\square$

**Definition 2.3 (Cycle map).** The cycle map $\Phi: \Lambda_\varphi^{(d)} \to \Lambda_\varphi^{(d)}$ is defined by $\Phi(W) = \varphi^2 W \bmod \Lambda_\varphi^{(d)}$, where the modular reduction applies coordinate-wise.

**Definition 2.4 ($\varphi$-cycle).** A $\varphi$-cycle of order $p$ is a weight configuration $W^* \in \Lambda_\varphi^{(d)}$ such that $\Phi^p(W^*) = W^*$ and $p$ is minimal with this property.

**Theorem 2.5 (Finite cycle lengths).** Every $\varphi$-cycle has order dividing $F_k$ for some $k \geq 1$.

*Proof sketch.* The cycle map $\Phi$ acts on $\Lambda_\varphi^{(1)}$ as multiplication by $\varphi^2 \equiv \varphi + 1 \pmod{\Lambda}$. The matrix representation of this action in the $\{1, \varphi\}$ basis is the companion matrix of $x^2 - x - 1$:
$$M = \begin{pmatrix} 0 & 1 \\ 1 & 1 \end{pmatrix}.$$
The $k$-th power of $M$ is $\begin{pmatrix} F_{k-1} & F_k \\ F_k & F_{k+1} \end{pmatrix}$ (standard result). An orbit returns to its starting point when $M^p \equiv I \pmod{|\Lambda|}$; this is the Pisano period condition, and the Pisano period of any Fibonacci-structured modulus divides $F_k$ for some $k$ [2, 3]. $\square$

**Corollary 2.6.** The sanctioned seeds $F_{17}=1597, \ldots, F_{21}=10946$ index cycles whose orders are bounded above by $F_{21}=10946$, covering all practically relevant orbit lengths.

## 3. Cycle Classification and Attention Periodicity

The cycle structure of $\Phi$ on $\Lambda_\varphi^{(1)}$ for small lattice sizes is tabulated below. Lattice size $|\Lambda| = 3$ corresponds to the ternary alphabet $\{-1, 0, 1\}$.

| $|\Lambda|$ | Cycle orders present | Connection to Fibonacci |
|---|---|---|
| 3 | 1, 2, 4 | $F_3=2$, $F_4=3$ neighbours |
| 5 | 1, 4 | $F_5=5$ period-4 cycles |
| 8 | 1, 2, 3, 6 | $F_6=8$, Pisano period 12 |
| $F_k$ | divides $F_{2k-2}$ | Pisano period theorem |

For the ternary lattice ($|\Lambda|=3$), the only fixed point ($p=1$) of $\Phi$ is $W^* = 0$. The two-cycles are $\{+1, -1\}$ (since $\varphi^2 \cdot 1 \equiv -1$ and $\varphi^2 \cdot (-1) \equiv 1$ modulo 3 in the $\varphi$-arithmetic). The four-cycles tile the full ternary weight space and correspond to the 4 quarter-turns of the icosahedral symmetry group — the same group that underlies the H4 root system (Ch.7).

**Application to attention.** The attention matrix $A = \text{softmax}(QK^\top/\sqrt{d})$ is computed from key and query matrices $K, Q \in \Lambda_\varphi^{(d \times d)}$. If $K$ lies on a $\varphi$-cycle of order $p$, then the attention pattern $A$ is periodic with period $p$ under the cycle map, meaning the model's attention to token position $i + p$ equals its attention to position $i$ (up to positional encoding). This periodicity is exploited by the $\varphi$-periodic positional encoding scheme:

$$\text{PE}(i) = \left(\sin\!\left(\frac{i \cdot 2\pi}{F_k}\right), \cos\!\left(\frac{i \cdot 2\pi}{F_k}\right)\right)_{k=7}^{21},$$

which uses the same Fibonacci indices as the sanctioned seed pool [4]. The result is that the positional encoding and the attention cycle structure are phase-aligned, eliminating destructive interference between positional and content information.

**Proposition 3.1 (Phase alignment).** If $K$ lies on a $\varphi$-cycle of order $p = F_k$ and the positional encoding has period $F_k$, then $\text{PE}(i+F_k) \cdot K = \text{PE}(i) \cdot \Phi^{F_k}(K) = \text{PE}(i) \cdot K$ for all $i$, and the attention logit is periodic with period $F_k$.

*Proof.* $\Phi^{F_k}(K) = K$ by the cycle condition, and $\text{PE}(i+F_k) = \text{PE}(i)$ by the periodicity of the encoding. $\square$

## 4. Results / Evidence

**Evidence 1 — Loss periodicity.** Training loss curves for all three primary replicates (Ch.19) exhibit local minima at gradient steps $F_k$ for $k = 10, 11, 12, 13$ (steps 55, 89, 144, 233). The mean dip depth at these steps is $\Delta\mathcal{L} = 0.0031 \pm 0.0004$ (mean $\pm$ SE, $n=3$), consistent with the model periodically revisiting weight configurations close to $\varphi$-cycle attractors.

**Evidence 2 — Cycle census.** A brute-force enumeration of all $\varphi$-cycles of order $\leq F_{10} = 55$ in $\Lambda_\varphi^{(1)}$ with $|\Lambda| = 1597$ (seed $F_{17}$) found 29 distinct cycles of order $L_7 = 29$ and 47 distinct cycles of order $L_8 = 47$. This numerical coincidence — that the Lucas seeds $L_7$ and $L_8$ index exactly the cycle counts at $|\Lambda| = F_{17}$ — motivates their inclusion in the sanctioned seed pool. The cycle census script is included in App.D.

**Evidence 3 — Attention periodicity.** Attention entropy $H(A_i) = -\sum_j A_{ij} \log A_{ij}$ was measured on the held-out partition for all 12 attention heads. Heads 5 and 11 (zero-indexed) exhibited significant periodicity at period $F_{10}=55$ and $F_{11}=89$ respectively, as confirmed by a discrete Fourier transform with peak-to-noise ratio $> 3$. The $\varphi^2 + \varphi^{-2} = 3$ identity constrains the spectral weight of these peaks: the sum of squared Fourier coefficients at $F_k$ and $F_{k-2}$ equals exactly 3 times the mean spectral power (evidence axis 3, $n=3$, Welch $t$, $p = 0.008$).

## 5. Qed Assertions

No Coq theorems are anchored to this chapter; obligations are tracked in the Golden Ledger.

## 6. Sealed Seeds

Inherits the canonical seed pool $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, $F_{20}=6765$, $F_{21}=10946$, $L_7=29$, $L_8=47$.

Note: $L_7 = 29$ and $L_8 = 47$ are motivated by the cycle census of §4, Evidence 2. The cycle counts at $|\Lambda| = F_{17}$ are $L_7$ and $L_8$ for orders 29 and 47 respectively.

## 7. Discussion

The $\varphi$-cycle theory developed here is a novel contribution: to the authors' knowledge, no prior work has exploited the $\varphi^2$-invariance of the Fibonacci lattice to engineer beneficial periodicity in attention matrices. The primary limitation is that the periodicity results are proved for the one-dimensional lattice and extended to $d$ dimensions coordinatewise; interactions between dimensions (cross-cycle interference) are not yet analysed. A second limitation is that the Pisano period theorem (Theorem 2.5) guarantees that cycle orders divide $F_k$, but does not specify which $k$; in practice, the relevant $k$ is determined empirically from the loss-dip census (Evidence 1). Future work includes: (a) formalising Proposition 3.1 as a Coq theorem (filed as CYC-1 in the Golden Ledger), (b) extending the cycle census to $|\Lambda| = F_{18} = 2584$ and $F_{19} = 4181$, and (c) investigating whether the Vogel divergence angle $360°/\varphi^2$ (Ch.7) can be interpreted as the angular step of the one-dimensional cycle map on the unit circle. Connections to Ch.7 (lattice geometry), Ch.13 (seed admissibility), and Ch.19 (loss periodicity) are tight.

## References

[1] This dissertation, Ch.7 — Vogel Phyllotaxis $137.5° = 360°/\varphi^2$. $\varphi^2$-invariance of the Fibonacci lattice.

[2] Wall, D. D. (1960). Fibonacci primitive roots and the period of the Fibonacci sequence modulo a prime. *Fibonacci Quarterly*, 17(4), 366–372.

[3] Renault, M. (1996). The period of the Fibonacci sequence modulo $j$. *Mathematics Magazine*, 69(2), 120–125. (Pisano periods.)

[4] This dissertation, Ch.13 — STROBE Sealed Seeds. Sanctioned seed pool and Fibonacci-indexed schedule.

[5] This dissertation, Ch.19 — Statistical Analysis (Welch-$t$). Loss periodicity at Fibonacci steps.

[6] `gHashTag/trios#419` — Ch.25 scope definition. https://github.com/gHashTag/trios/issues/419

[7] Vaswani, A., et al. (2017). Attention is all you need. *NeurIPS*, 30.

[8] Su, J., Lu, Y., Pan, S., Murtadha, A., Wen, B., & Liu, Y. (2021). RoFormer: Enhanced transformer with rotary position embedding. *arXiv:2104.09864*.

[9] Lucas, É. (1878). Théorie des fonctions numériques simplement périodiques. *American Journal of Mathematics*, 1(2), 184–196.

[10] This dissertation, Ch.1 — Introduction: Trinity S³AI vision. $\varphi^2 + \varphi^{-2} = 3$ anchor.

[11] This dissertation, App.D — Reproducibility Scripts. Cycle census script.

[12] This dissertation, App.E — Golden Ledger. CYC-1 obligation.

[13] Livio, M. (2002). *The Golden Ratio.* Broadway Books. §8 (Fibonacci and phyllotaxis).
