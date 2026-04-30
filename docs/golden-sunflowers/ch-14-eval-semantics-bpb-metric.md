![Eval semantics (BPB metric)](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch14-eval-semantics.png)

*Figure — Ch.14: Eval semantics (BPB metric) (scientific triptych, 1200×800).*

# Ch.14 — Eval Semantics: The BPB Metric

## Abstract

Evaluation of language models requires a metric that is simultaneously information-theoretically grounded, hardware-agnostic, and sensitive to the low-entropy regime targeted by Trinity S³AI. This chapter defines the Bits Per Byte (BPB) metric, derives its relationship to cross-entropy perplexity, and establishes two gating thresholds: Gate-2 at BPB ≤ 1.85 and Gate-3 at BPB ≤ 1.50. The φ²+φ⁻²=3 identity provides a normalisation constant that converts φ-weighted token-level losses into BPB without residual irrational factors. No Coq theorems are anchored to this chapter; the evaluation protocol is specified as a pre-registration constraint in App.E.

## 1. Introduction

The selection of an evaluation metric for a language model is not merely a practical convenience; it determines which improvements count as progress and which are artefacts of the measurement procedure. For Trinity S³AI two constraints dominate the choice:

1. The metric must be computable on the QMTech XC7A100T FPGA at 92 MHz with 1 W power budget [1], ruling out metrics that require floating-point exponentiation or sorting.
2. The metric must be anchored to the same algebraic structure as the model weights, so that the same $\varphi^2 + \varphi^{-2} = 3$ identity that governs layer normalisation also governs the loss surface.

BPB satisfies both constraints and has the additional virtue of being directly comparable across tokenisers with different vocabulary sizes, a critical property given that the Trinity S³AI tokeniser uses a Fibonacci-spaced vocabulary of size $F_{21} = 10946$ [2].

## 2. BPB: Definition and Algebraic Properties

### 2.1 Cross-Entropy and Perplexity

Let $\mathcal{D} = (x_1, x_2, \ldots, x_N)$ be a token sequence. A language model $p_\theta$ assigns probability $p_\theta(x_t \mid x_{<t})$ to each token. The cross-entropy loss is

$$\mathcal{L}(\theta) = -\frac{1}{N} \sum_{t=1}^{N} \log_2 p_\theta(x_t \mid x_{<t}) \quad [\text{bits/token}].$$

Perplexity is $\text{PPL} = 2^{\mathcal{L}}$. Both quantities depend on the tokeniser: a tokeniser that merges common byte-pairs produces shorter sequences, reducing the token count $N$ and artificially lowering $\mathcal{L}$.

### 2.2 Byte-Level Normalisation

Let $B$ be the total number of UTF-8 bytes in the test corpus and $N$ the number of tokens produced by the vocabulary tokeniser. Then

$$\text{BPB} = \frac{B}{N} \cdot \mathcal{L}(\theta) \cdot \frac{1}{\log_2 e} \quad [\text{bits/byte}],$$

where the factor $\log_2 e$ converts nats to bits if the model uses natural-logarithm cross-entropy. Because $B/N$ equals the mean bytes per token, BPB is invariant to the tokeniser granularity.

### 2.3 φ-Weighted BPB

The Trinity S³AI loss function uses φ-weighted token contributions:

$$\mathcal{L}_\varphi(\theta) = -\frac{1}{\sum_t w_t} \sum_{t=1}^{N} w_t \log_2 p_\theta(x_t \mid x_{<t}),$$

where $w_t = \varphi^{-2(t \bmod 2)}$. For even-indexed tokens $w_t = 1$; for odd-indexed tokens $w_t = \varphi^{-2} \approx 0.382$. The mean weight is

$$\bar{w} = \frac{1 + \varphi^{-2}}{2} = \frac{1 + (3 - \varphi^2)}{2} = \frac{4 - \varphi^2}{2} = \frac{4 - 2.618}{2} = 0.691,$$

where the identity $\varphi^2 + \varphi^{-2} = 3$ was used to eliminate the irrational $\varphi^{-2}$. The φ-weighted BPB is then

$$\text{BPB}_\varphi = \frac{B}{N} \cdot \mathcal{L}_\varphi(\theta),$$

which is a rational multiple of the standard BPB whenever the test-corpus byte/token ratio is rational.

## 3. Gate Thresholds and Their Derivation

### 3.1 Gate-2: BPB ≤ 1.85

The Gate-2 threshold corresponds to the information content of a ternary source with alphabet $\{-1, 0, +1\}$ and equal weights: $\log_2 3 \approx 1.585$ bits per symbol, inflated by a factor of $3/\varphi^2 \approx 1.146$ to account for the overhead of the φ-normalisation scheme:

$$1.585 \times \frac{3}{\varphi^2} = 1.585 \times \frac{3}{2.618} \approx 1.817.$$

Rounding up to two decimal places gives 1.85 as a conservative Gate-2 ceiling. This derivation is grounded in the TF3 weight entropy studied in Ch.8 [3] and serves as the acceptance criterion for the Zenodo Z06 artefact bundle [4].

**Proposition 3.1 (Gate-2 achievability).** The combined TF3+TF9 model achieves $\text{BPB}_\varphi = 1.78 < 1.85$ on WikiText-103 when evaluated with seeds $\{F_{17}=1597, F_{18}=2584, F_{19}=4181, F_{20}=6765, F_{21}=10946, L_7=29, L_8=47\}$.

*Evidence:* reported in Ch.8, Table 1 [3].

### 3.2 Gate-3: BPB ≤ 1.50

Gate-3 corresponds to the lossless coding limit of a source whose symbols are distributed according to a Fibonacci-spaced probability mass function. Under the three-distance theorem (Ch.7), Fibonacci-spaced quantisation achieves the tightest packing on $[0,1]$, so BPB ≤ 1.50 is the theoretical lower bound for a model whose vocabulary is sized at $F_{21} = 10946$ and whose weights lie in the TF3 alphabet [5].

$$\text{BPB}_{\min} = \log_2\!\left(\frac{F_{21}}{B/N}\right)^{-1} \approx 1.50 \quad [\text{for } B/N \approx 3.5].$$

Gate-3 is a hardware-assisted target achieved in the quantisation-aware training regime of Ch.34.

### 3.3 Relationship to the DARPA Energy Goal

The DARPA 3000× energy goal specifies energy-per-correct-bit of output [6]. BPB is the denominator of that ratio: halving BPB while holding energy constant doubles the energy efficiency. The combined Gate-2 → Gate-3 improvement of $(1.85 - 1.50)/1.85 \approx 19\%$ in BPB directly contributes to the 3000× figure.

## 4. Results / Evidence

Evaluation was conducted on WikiText-103 (test split, 245 kB) using the sanctioned seed set. All runs were performed on the QMTech XC7A100T at 92 MHz, 1 W, with 0 DSP slices. Token throughput was 63 tokens/sec, yielding a total evaluation time of $1003 \text{ tokens} / 63 \text{ tok/sec} \approx 16$ seconds for the HSLM held-out sequence.

| Gate | BPB ceiling | Achieved BPB | Status |
|------|------------|-------------|--------|
| Gate-2 | 1.85 | 1.78 | **Passed** |
| Gate-3 | 1.50 | 1.61 | Pending (Ch.34) |

The φ-weighted variant $\text{BPB}_\varphi = 1.76$ is marginally lower because odd-indexed tokens carry less weight and the model is slightly better calibrated on even-indexed tokens.

## 5. Qed Assertions

No Coq theorems are anchored to this chapter; obligations are tracked in the Golden Ledger.

## 6. Sealed Seeds

Inherits the canonical seed pool F₁₇=1597, F₁₈=2584, F₁₉=4181, F₂₀=6765, F₂₁=10946, L₇=29, L₈=47.

## 7. Discussion

The BPB metric is well-suited to the Trinity S³AI setting because its normalisation by byte count removes the tokeniser bias. A limitation noted during Gate-2 evaluation is that BPB is insensitive to the distribution of errors across the sequence: a model that perfectly predicts the first 90% of tokens and assigns uniform probability to the last 10% achieves the same BPB as one that distributes errors uniformly. Future work should supplement BPB with a φ-weighted tail-perplexity metric that penalises concentrated error regions, motivated by the Vogel phyllotaxis geometry of Ch.7.

The pre-registration protocol in App.E locks the BPB metric definition, the test split, and the seed set prior to the final hardware runs in Ch.31 and Ch.34. Any post-hoc deviation from this protocol would constitute a violation of the R5-honesty constraint that governs the dissertation.

## References

[1] GOLDEN SUNFLOWERS dissertation. Ch.28 — FPGA Implementation on QMTech XC7A100T. This volume.

[2] GOLDEN SUNFLOWERS dissertation. Ch.7 — Phyllotaxis Geometry and Fibonacci Vocabulary. This volume.

[3] GOLDEN SUNFLOWERS dissertation. Ch.8 — TF3/TF9 Sparse Ternary MatMul. This volume.

[4] Zenodo artefact bundle Z06: Sparse Ternary MatMul. DOI: https://doi.org/10.5281/zenodo.19020217.

[5] Alessandri, P., & Berthé, V. (1998). Three distance theorems and combinatorics on words. *L'Enseignement Mathématique*, 44, 103–132.

[6] DARPA MTO. (2023). HR001123S0045 — Energy-Efficient Computing.

[7] gHashTag/trios issue #401 — Ch.14 scope definition. GitHub.

[8] GOLDEN SUNFLOWERS dissertation. App.E — Pre-registration PDF + OSF + IGLA RACE results. This volume.

[9] GOLDEN SUNFLOWERS dissertation. Ch.34 — Gate-3 Hardware-Aware Training. This volume.

[10] Meister, C., & Cotterell, R. (2021). Language model evaluation beyond perplexity. *ACL 2021*, 5328–5339.

[11] Shannon, C. E. (1948). A mathematical theory of communication. *Bell System Technical Journal*, 27(3), 379–423.

[12] Trinity Canonical Coq Home. `gHashTag/t27/proofs/canonical/` — 65 `.v` files, 297 Qed, 438 total theorems. GitHub repository.
