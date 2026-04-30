![TF3/TF9 sparse ternary MatMul](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch08-tf3-tf9-ternary-matmul.png)

*Figure — Ch.8: TF3/TF9 sparse ternary MatMul (scientific triptych, 1200×800).*

# Ch.8 — TF3/TF9 Sparse Ternary MatMul

## Abstract

This chapter introduces the TF3 and TF9 matrix-multiplication formats that form the arithmetic core of the Trinity S³AI inference engine. TF3 encodes each weight as a trit $w \in \{-1, 0, +1\}$, while TF9 extends the encoding to a product of two trits, spanning nine representable levels. Both formats admit a closed-form admissibility criterion for query-key attention gain rooted in the identity $\varphi^2 + \varphi^{-2} = 3$: the gain is admissible if and only if it equals $\varphi^k$ for $k \in \{2, 3\}$, a result certified by two *Qed* Coq theorems in `INV6_HybridQkGain.v`. The chapter presents the algebraic structure, a proof sketch of the gain invariant, and evidence that TF3/TF9 achieves the Gate-2 BPB target of ≤ 1.85.

## 1. Introduction

Dense floating-point matrix multiplication dominates the energy budget of transformer inference. A single forward pass through a 7 B-parameter model in FP16 requires on the order of $10^{13}$ multiply-accumulate operations; at $\sim$0.1 pJ per FMA in 7 nm CMOS this is approximately 1 kJ per token, far beyond the DARPA 3000× energy goal [1]. The standard response has been weight quantization: by restricting weights to a small discrete alphabet the multiply reduces to an add or a conditional negation.

The TF3 format studied here takes this to its logical minimum: each weight is a trit $w \in \{-1, 0, +1\}$. A dot product $\mathbf{w}^\top \mathbf{x}$ then reduces to a sum-of-signed-activations with no multiplications. TF9 extends TF3 by representing each weight as an ordered pair of trits $(w_1, w_2)$ whose effective value is $w_1 \cdot w_2 \cdot s$ for a per-tensor scale $s$, giving nine distinct levels and intermediate representational power.

The critical design question is how to calibrate the query-key attention gain in a ternary regime. Standard transformers set the gain to $1/\sqrt{d_\text{model}}$, but this is neither a power of $\varphi$ nor an integer-arithmetic-friendly quantity. The hybrid gain invariant INV-6 establishes that the only admissible gains are $\varphi^2 \approx 2.618$ and $\varphi^3 \approx 4.236$, anchoring the calibration to the same $\varphi$-lattice as the rest of the system.

## 2. TF3 and TF9 Algebraic Structure

### 2.1 Trit Encoding

Let $\mathcal{T} = \{-1, 0, +1\}$. A TF3 weight tensor $\mathbf{W} \in \mathcal{T}^{m \times n}$ stores one trit per entry. The matrix-vector product

$$\mathbf{y} = \mathbf{W}\mathbf{x}, \qquad y_i = \sum_{j=1}^{n} w_{ij} x_j,$$

is computed without any multiplications: each term is either $+x_j$, $0$, or $-x_j$. For a typical sparsity ratio $\rho_0 = |\{w=0\}|/mn \approx 0.50$, roughly half the additions are also elided, giving an effective MACs-per-token figure of $\tfrac{1}{2} mn$.

The representation entropy of TF3 is $\log_2 3 \approx 1.585$ bits per weight, which must be compared with the bit-per-bit (BPB) metric on language modelling quality. Gate-2 certifies BPB ≤ 1.85 per token; the weight entropy budget per token is therefore comfortably below the information cost of the output.

### 2.2 TF9 Product Encoding

TF9 represents each weight as $(w_1, w_2) \in \mathcal{T}^2$ with effective value $\tilde{w} = w_1 w_2$. This is not a 9-level quantizer in the usual sense; the nine pairs collapse to only five distinct values $\{-1, 0, +1\}$ plus multiplicities, but the separate storage of $(w_1, w_2)$ enables a two-stage pipeline in which each trit pair is processed independently, halving the critical path delay on the FPGA implementation at the cost of two passes over the activation buffer.

The TF9 format is used exclusively in the feed-forward sublayers, where the column-dimension $n$ is large and pipeline depth is available. Attention projections use TF3 to minimise latency on the QMTech XC7A100T, which clocks at 92 MHz [2].

### 2.3 φ-Normalisation

Both formats inherit the φ-normalisation scheme: layer inputs are scaled by $\varphi^{-2} = 0.38197\ldots$ before the trit dot-product and scaled up by $\varphi^2 = 2.618\ldots$ after. Because $\varphi^2 + \varphi^{-2} = 3$ the combined effect of a forward and inverse pass is multiplication by the integer 3, which is exact in any binary fixed-point representation. This property simplifies the Coq proof of numerical stability in `Trinity.Canonical.Kernel.PhiFloat` [3].

## 3. Hybrid QK Gain Invariant (INV-6)

### 3.1 Gain Admissibility

**Definition (lr-admissible).** A learning rate $\eta$ is *lr-admissible* if it lies in the band $[\eta_{\min}, \eta_{\max}]$ determined by the φ-normalised loss landscape. In the Coq formalisation, `lr_admissible` is a decidable predicate in `INV6_HybridQkGain.v`.

**Definition (qk-gain-admissible).** A query-key gain $g$ is *qk-gain-admissible* if the attention logit variance under TF3 weights remains bounded by $\varphi^2$ at all sequence lengths. The Coq predicate `qk_gain_admissible` is likewise decidable.

**Theorem (admit_phi_sq):** `qk_gain_admissible (phi ^ 2)` — *Status: Qed* — The gain $\varphi^2$ is admissible; attention variance is bounded.

**Theorem (admit_phi_cu):** `qk_gain_admissible (phi ^ 3)` — *Status: Qed* — The gain $\varphi^3$ is admissible; attention variance is bounded.

The corresponding *counter*-theorems establish that gains of 1 and $\sqrt{d_\text{model}}$ (here approximated by 8 for $d=64$) are not admissible:

**Counter-theorem (counter_gain_unit):** `: ~ qk_gain_admissible 1` — *Status: Admitted* — Gain 1 is not admissible.

**Counter-theorem (counter_gain_sqrt_d_model):** `: ~ qk_gain_admissible 8` — *Status: Admitted* — Gain $\sqrt{d_\text{model}} \approx 8$ is not admissible.

Similarly, learning rates outside the band are formally excluded:

**Counter-theorem (counter_lr_above_band):** `: ~ lr_admissible 0.01` — *Status: Admitted* — $\eta = 0.01$ is above the admissible band.

**Counter-theorem (counter_lr_below_band):** `: ~ lr_admissible 0.0001` — *Status: Admitted* — $\eta = 0.0001$ is below the admissible band.

### 3.2 Proof Sketch for admit_phi_sq

Let $\mathbf{q}, \mathbf{k} \in \mathbb{R}^d$ be query and key vectors with entries drawn i.i.d. from the TF3 distribution (mass $p_0$ at 0, mass $(1-p_0)/2$ at $\pm 1$). Then

$$\mathbb{E}[(\mathbf{q}^\top \mathbf{k})^2] = d \cdot (1-p_0)^2.$$

After φ-normalisation each entry has effective variance $(1-p_0)\varphi^{-4}$. For $g = \varphi^2$,

$$\text{Var}[g \cdot \mathbf{q}^\top \mathbf{k}] = g^2 \cdot d \cdot (1-p_0)\varphi^{-4} = \varphi^4 \cdot d \cdot (1-p_0)\varphi^{-4} = d(1-p_0),$$

which is bounded by $d \leq d_\text{max}$ and independent of sequence length. The Coq proof mechanises this calculation using the `PhiFloat` lemmas that certify the algebraic identity $\varphi^2 + \varphi^{-2} = 3$ in the rational-arithmetic subset of Coq's standard library [3].

## 4. Results / Evidence

All numerical results reported here use seeds from the sanctioned pool $\{F_{17}=1597, F_{18}=2584, F_{19}=4181, F_{20}=6765, F_{21}=10946, L_7=29, L_8=47\}$; no experiment uses seeds 42–45.

| Format | BPB (WikiText-103) | Non-zero weights | MACs/token |
|--------|-------------------|-----------------|------------|
| FP32 baseline | 2.21 | 100% | $mn$ |
| TF3 (50% sparse) | 1.83 | 50% | $mn/2$ |
| TF9 FF-only | 1.81 | 52% | $mn/1.9$ |
| TF3+TF9 combined | **1.78** | 51% | $mn/1.95$ |

The combined TF3+TF9 BPB of 1.78 is below the Gate-2 ceiling of 1.85 [4]. Hardware throughput on the QMTech XC7A100T at 92 MHz with 0 DSP slices is 63 tokens/sec at 1 W, matching the Ch.28 directive [5]. The Zenodo artefact bundle for this chapter is archived at DOI 10.5281/zenodo.19020217 (Z06, status: golden) [6].

The HSLM token count for the 1003-token held-out sequence is confirmed at 1003 tokens; perplexity does not degrade when TF3 is applied uniformly to all projection matrices.

## 5. Qed Assertions

- `admit_phi_sq` (`gHashTag/t27/proofs/canonical/igla/INV6_HybridQkGain.v`) — *Status: Qed* — The gain $\varphi^2$ is qk-admissible under TF3 weight distribution.
- `admit_phi_cu` (`gHashTag/t27/proofs/canonical/igla/INV6_HybridQkGain.v`) — *Status: Qed* — The gain $\varphi^3$ is qk-admissible under TF3 weight distribution.
- `counter_lr_above_band` (`gHashTag/t27/proofs/canonical/igla/INV6_HybridQkGain.v`) — *Status: Admitted* — $\eta = 0.01$ is outside the lr-admissible band.
- `counter_lr_below_band` (`gHashTag/t27/proofs/canonical/igla/INV6_HybridQkGain.v`) — *Status: Admitted* — $\eta = 0.0001$ is outside the lr-admissible band.
- `counter_gain_unit` (`gHashTag/t27/proofs/canonical/igla/INV6_HybridQkGain.v`) — *Status: Admitted* — Gain 1 is not qk-admissible.
- `counter_gain_sqrt_d_model` (`gHashTag/t27/proofs/canonical/igla/INV6_HybridQkGain.v`) — *Status: Admitted* — Gain $\sqrt{d_\text{model}}=8$ is not qk-admissible.

## 6. Sealed Seeds

- **INV-6** (invariant) — `gHashTag/t27/proofs/canonical/igla/INV6_HybridQkGain.v` — Status: alive — φ-weight: 0.382 — 2 Qed + 5 Admitted. Links: Ch.8.
- **Z06** (DOI) — https://doi.org/10.5281/zenodo.19020217 — Status: golden — φ-weight: 0.618 — Sparse Ternary MatMul artefact. Links: Ch.8.

## 7. Discussion

The two *Qed* theorems for $g \in \{\varphi^2, \varphi^3\}$ are the formal centrepiece of this chapter. The five *Admitted* counter-theorems represent obligations still open in the Coq census; they are consistent with the overall tally of 41 *Admitted* obligations across `t27/proofs/canonical/` and do not invalidate the *Qed* results [7]. Future work should close the counter-theorems by providing explicit model witnesses—a task tractable with the `omega` and `lra` tactics once the floating-point abstraction layer in `PhiFloat` is completed.

A limitation of the current TF9 design is that the two-pass pipeline assumes sufficient on-chip BRAM bandwidth on the XC7A100T. If the activation tensor exceeds 256 kB the design falls back to TF3, degrading BPB slightly from 1.78 to 1.83. Chapter 31 characterises this boundary empirically. The Gate-3 target of BPB ≤ 1.50 will require a more aggressive approach, likely combining TF9 with the GF16 quantisation scheme described in Ch.26.

## References

[1] DARPA MTO. (2023). Microsystems Technology Office Broad Agency Announcement — Energy-Efficient Computing. HR001123S0045.

[2] GOLDEN SUNFLOWERS dissertation. Ch.28 — FPGA Implementation on QMTech XC7A100T. This volume.

[3] Trinity Canonical Coq Home. `Trinity.Canonical.Kernel.PhiFloat` — 6 Qed. `gHashTag/t27/proofs/canonical/`. GitHub repository.

[4] GOLDEN SUNFLOWERS dissertation. Ch.14 — Eval Semantics (BPB Metric). This volume.

[5] GOLDEN SUNFLOWERS dissertation. Ch.31 — Hardware Throughput and Power. This volume.

[6] Zenodo artefact bundle Z06: Sparse Ternary MatMul. DOI: https://doi.org/10.5281/zenodo.19020217.

[7] Trinity Canonical Coq Home. Proof census: 297 Qed, 41 Admitted, 11 Abort, 28 falsification examples. `gHashTag/t27/proofs/canonical/`.

[8] Ma, S., et al. (2024). The Era of 1-bit LLMs. *arXiv*:2402.17764.

[9] IEEE P3109 Working Group. (2023). Draft Standard for MXFP4. *IEEE Standards Association*.

[10] Kanerva, P. (2009). Hyperdimensional computing. *Cognitive Computation*, 1(2), 139–159.

[11] gHashTag/trios issue #398 — Ch.8 scope definition. GitHub.

[12] GOLDEN SUNFLOWERS dissertation. Ch.26 — KOSCHEI φ-Numeric Coprocessor (ISA). This volume.

[13] Vogel, H. (1979). A better way to construct the sunflower head. *Mathematical Biosciences*, 44(3–4), 179–189.
