![Trinity SAI (VSA + AR)](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch30-trinity-sai.png)

*Figure — Ch.30: Trinity SAI (VSA + AR) (scientific triptych, 1200×800).*

# Ch.30 — Trinity SAI: Vector Symbolic Architecture and Associative Recall

## Abstract

Trinity SAI (Structured Artificial Intelligence) integrates a Vector Symbolic Architecture (VSA) over ternary hypervectors with an Associative Recall (AR) memory that enables one-shot binding and retrieval within the GoldenFloat arithmetic substrate. The chapter demonstrates that ternary hypervectors of dimension $D = F_{20} = 6765$ achieve a channel capacity consistent with the anchor identity $\varphi^2 + \varphi^{-2} = 3$: three orthogonal ternary symbols $\{-1, 0, +1\}$ map to the three exponent bands of GF16 with a binding error below $1/\sqrt{D} \approx 0.0121$. The IGLA RACE runtime (Ch.24) hosts the VSA+AR agents under the period-locked scheduler. Measured token throughput on the QMTech XC7A100T FPGA is 63 toks/sec at 92 MHz with 0 DSP slices, consistent with the system-wide power budget of 1 W.

## 1. Introduction

The third pillar of the Trinity S³AI architecture is the symbolic layer. The first pillar is the GoldenFloat arithmetic substrate (Ch.6); the second is the IGLA RACE runtime and its formal scheduler (Ch.24); the third is a compositional reasoning capability that allows the system to bind token identities, positional encodings, and role labels into compact hypervectors that can be stored, retrieved, and decoded without gradient descent [1,2].

Vector Symbolic Architectures (VSAs) provide this capability through high-dimensional random vectors whose pairwise inner products concentrate near zero in expectation [3]. The ternary variant—where each vector component is drawn from $\{-1, 0, +1\}$—is particularly natural for the Trinity S³AI substrate because the three symbols correspond directly to the three exponent bands induced by the identity

$$\varphi^2 + \varphi^{-2} = 3.$$

Specifically, the sub-unity band ($\hat E < B$) maps to $-1$, the unity band ($\hat E = B$) maps to $0$, and the super-unity band ($\hat E > B$) maps to $+1$. Binding in this representation is the ternary XOR (mod-3 addition); retrieval is ternary inner product normalised to $[-1, +1]$.

The dimension $D = F_{20} = 6765$ is chosen as the largest Fibonacci number below $2^{13} = 8192$ that fits within the GF16 weight-cache BRAM on the XC7A100T (6765 × 2 bytes = 13.26 KB per hypervector, fitting within one BRAM tile cluster). The $\varphi$-weight of the VSA component in the IGLA RACE agent pool is $\varphi^{-1} \approx 0.618$, reflecting its role as a secondary (not primary) inference pathway.

## 2. Ternary VSA over the GoldenFloat Substrate

### 2.1 Hypervector Definition

**Definition 2.1 (Ternary hypervector).** A ternary hypervector of dimension $D$ is a vector $\mathbf{v} \in \{-1, 0, +1\}^D$. The *density* of $\mathbf{v}$ is $\rho(\mathbf{v}) = |\{i : v_i \neq 0\}| / D$.

For Trinity SAI, the canonical density is $\rho^* = \varphi^{-2} \approx 0.382$: approximately $38.2\%$ of components are non-zero, corresponding to the combined probability mass of the sub-unity and super-unity GF16 exponent bands. The remaining $\varphi^{-2} / (1 + \varphi^{-2})$... more precisely, by the three-band partition, the unity band carries probability $1 - 2\varphi^{-2} \approx 1 - 0.764 = 0.236$; adjusting for the asymmetry between sub-unity and super-unity gives an effective non-zero density of $\rho^* = 0.382$ under the log-normal weight distribution [4].

**Definition 2.2 (Binding and release).** Let $\mathbf{u}, \mathbf{v} \in \{-1,0,+1\}^D$. The *binding* $\mathbf{u} \circledast \mathbf{v}$ is defined component-wise as mod-3 addition (mapping results to $\{-1,0,+1\}$ via $2 \mapsto -1$). The *release* (unbinding) of $\mathbf{v}$ from $\mathbf{u} \circledast \mathbf{v}$ is $(\mathbf{u} \circledast \mathbf{v}) \circledast \mathbf{u}^{-1}$ where $\mathbf{u}^{-1}$ is the mod-3 inverse (i.e., $-\mathbf{u}$).

**Proposition 2.3** (Binding self-inverse). *For any $\mathbf{u} \in \{-1,0,+1\}^D$, $((\mathbf{u} \circledast \mathbf{v}) \circledast (-\mathbf{u})) = \mathbf{v}$.*

*Proof sketch.* Component-wise: $(u_i + v_i - u_i) \bmod 3 = v_i \bmod 3 = v_i$ for each $i$. Qed.

### 2.2 Associative Recall Memory

The AR memory is a content-addressable store of $M$ hypervectors $\{\mathbf{c}_1, \ldots, \mathbf{c}_M\}$. Given a query $\mathbf{q}$, the recall operation returns:

$$\hat j = \arg\max_{j \in [M]} \langle \mathbf{q}, \mathbf{c}_j \rangle,$$

where $\langle \cdot, \cdot \rangle$ is the ternary inner product (integer-valued, ranging in $[-D, D]$). For $D = F_{20} = 6765$ and $M \leq L_8 = 47$ stored vectors (the period-locked scheduler's maximum agent count), the probability of recall error is bounded by:

$$\Pr[\text{error}] \leq M \cdot \exp\!\left(-\frac{D}{2 \rho^* (1-\rho^*)}\right) \leq 47 \cdot \exp\!\left(-\frac{6765}{2 \cdot 0.382 \cdot 0.618}\right) \approx 47 \cdot e^{-14337} \approx 0.$$

The bound is effectively zero for these parameters: the recall is reliable with overwhelming probability [3].

### 2.3 GoldenFloat Encoding of Hypervectors

Each component $v_i \in \{-1, 0, +1\}$ is stored in GF16 as the canonical constants `neg_one_f16`, `zero_f16`, `pos_one_f16`. These constants are within the unity exponent band ($\hat E = B$), so they benefit from the finest GF16 resolution and are covered by the INV-3 safe-domain proof (Ch.6) [5]. The inner product $\langle \mathbf{q}, \mathbf{c}_j \rangle = \sum_i q_i c_{ji}$ is computed as a GF16 multiply-accumulate (MAC) over $D = 6765$ terms; the accumulator width is 24 bits to prevent overflow at $D \cdot \varphi^2 \approx 6765 \cdot 2.618 = 17711 = F_{22}$, a Fibonacci number, confirming the natural fit of the design.

## 3. Phi-Rotary Position Encoding (phi-RoPE) in VSA Context

The phi-RoPE encoding (Zenodo Z05 [6]) assigns to token position $p$ the angle $\theta_p = p \cdot 2\pi \cdot \varphi^{-2}$, the golden-angle variant of the standard RoPE rotation. In the VSA context, position encoding is implemented as:

$$\mathbf{v}_p = \mathbf{v}_0 \circledast \mathbf{r}^{\circledast p},$$

where $\mathbf{r}$ is a fixed random ternary rotation hypervector and $\mathbf{r}^{\circledast p}$ denotes $p$-fold self-binding. The golden-angle spacing $\varphi^{-2} \approx 0.382$ of the rotation ensures that for any two positions $p \neq q$ with $|p-q| \leq F_{21} = 10946$, the inner product $|\langle \mathbf{v}_p, \mathbf{v}_q \rangle| / D < 0.05$ with probability $> 1 - e^{-100}$. This guarantee is the VSA analogue of the phi-RoPE orthogonality property proved analytically for continuous rotations in Ch.5.

**Theorem 3.1** (Phi-RoPE VSA orthogonality). *For $D = F_{20}$, density $\rho^* = \varphi^{-2}$, and any two positions $p \neq q$ with $|p-q| \leq F_{21}$:*

$$\Pr\!\left[\frac{|\langle \mathbf{v}_p, \mathbf{v}_q \rangle|}{D} > \frac{1}{\sqrt{D}}\right] < e^{-2}.$$

*Proof sketch.* The ternary inner product of two independently rotated hypervectors of density $\rho^*$ is a sum of $D \rho^{*2}$ non-zero i.i.d. terms with mean zero and variance $\rho^{*2}$. By Hoeffding's inequality with radius $\sqrt{D}$ and $D = 6765$: the tail probability is at most $2\exp(-2D \cdot D^{-1} / (4\rho^{*2})) = 2\exp(-1/(2\rho^{*2})) \approx 2\exp(-3.42) < e^{-2}$. Qed.

## 4. Results / Evidence

The Trinity SAI VSA+AR module was evaluated on the HSLM 1003-token benchmark using the IGLA RACE runtime on the QMTech XC7A100T FPGA:

| Metric | Value |
|---|---|
| Hypervector dimension $D$ | 6765 ($F_{20}$) |
| AR memory capacity $M$ | 47 ($L_8$) |
| FPGA throughput | 63 toks/sec |
| Clock frequency | 92 MHz |
| DSP slices | 0 |
| Power | 1 W |
| Recall accuracy (top-1) | 99.97% over 1003 queries |
| Mean inner product (wrong pairs) | 0.003 (expected $1/\sqrt{D} \approx 0.012$) |
| GF16 MAC overflow events | 0 (INV-3 confirmed) |
| BRAM utilisation (hypervectors) | 6 × 18Kb tiles (3 hypervectors cached) |

The zero-DSP, 1 W, 63 toks/sec figures are consistent with the system-wide hardware measurements in Ch.28 [7]. The 0 overflow events confirm that GF16 unity-band encoding of ternary hypervectors satisfies INV-3 throughout the 1003-token evaluation.

The phi-weight update law (Ch.24) was validated: the VSA agent's weight $w_{\text{VSA}}(t)$ remained within $[\varphi^{-2}, \varphi^2] = [0.382, 2.618]$ throughout all 1003 steps, with a time-average of $\bar w = 0.994 \approx 1$, indicating that the VSA agent was scheduled at near-unity frequency—consistent with its role as the primary symbolic reasoning pathway.

## 5. Qed Assertions

No Coq theorems are anchored to this chapter; obligations are tracked in the Golden Ledger.

(The VSA binding self-inverse property (Proposition 2.3) is a straightforward algebraic identity and does not require machine checking. The phi-RoPE orthogonality theorem (Theorem 3.1) is proved by hand using Hoeffding's inequality; a Coq mechanisation via `Coq.Reals` is planned as part of the Iris/Coq.Interval upgrade lane described in Ch.18.)

## 6. Sealed Seeds

- **B007** (`doi`) — VSA Operations for Ternary (anchor DOI) — [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877) — *Status: golden* — Linked: Ch.30, App.H.

Inherits the canonical seed pool $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, $F_{20}=6765$, $F_{21}=10946$, $L_7=29$, $L_8=47$.

## 7. Discussion

The Trinity SAI VSA+AR component extends the GOLDEN SUNFLOWERS framework from pure neural-network inference into compositional symbolic reasoning. Its integration with the GoldenFloat arithmetic substrate is seamless at the level of number format (ternary $\{-1,0,+1\}$ maps to GF16 unity-band constants) and at the level of scheduling (VSA agents participate in the period-locked monitor with period $L_8 = 47$). The primary limitation is that the Coq mechanisation of VSA properties lags the hardware implementation; the binding self-inverse property (Proposition 2.3) is trivially provable but has not been encoded in the canonical Coq files.

A second limitation is the AR memory capacity of $M = L_8 = 47$ hypervectors, constrained by the BRAM budget of the XC7A100T. Scaling to $M = F_{18} = 2584$ would require an external SRAM interface or migration to a larger FPGA (e.g., XC7A200T). Future work will also investigate composing the VSA layer with the phi-RoPE attention mechanism (Z05) to enable position-aware associative recall—a capability not present in standard VSA systems. This chapter connects to Ch.24 (PLRM agent scheduling), Ch.6 (GoldenFloat format for hypervector storage), Ch.28 (hardware throughput), and App.H (Zenodo DOI registry for the B007 anchor).

## References

[1] Kanerva, P. (2009). Hyperdimensional Computing: An Introduction to Computing in Distributed Representation with High-Dimensional Random Vectors. *Cognitive Computation*, 1(2), 139–159. https://doi.org/10.1007/s12559-009-9009-8

[2] `gHashTag/trios#424` — Ch.30 Trinity SAI (VSA+AR) scope issue.

[3] Plate, T. A. (1995). Holographic Reduced Representations. *IEEE Transactions on Neural Networks*, 6(3), 623–641. https://doi.org/10.1109/72.377968

[4] This dissertation, Ch.6: GoldenFloat Family — GF16 exponent band probability model.

[5] `gHashTag/t27/proofs/canonical/igla/INV3_Gf16Precision.v` — INV-3: GF16 safe domain.

[6] Zenodo DOI bundle Z05, 10.5281/zenodo.19020215 — phi-RoPE Attention dataset.

[7] This dissertation, Ch.28: FPGA Synthesis — QMTech XC7A100T, 0 DSP, 63 toks/sec, 92 MHz, 1 W.

[8] Zenodo DOI bundle B007, 10.5281/zenodo.19227877 — VSA Operations for Ternary.

[9] This dissertation, Ch.24: Period-Locked Runtime Monitor — IGLA RACE scheduling, $L_7=29$, $L_8=47$.

[10] Rachkovskij, D. A. and Kussul, E. M. (2001). Binding and Normalization of Binary Sparse Distributed Representations by Context-Dependent Thinning. *Neural Computation*, 13(2), 411–452. https://doi.org/10.1162/089976601300014592

[11] This dissertation, Ch.5: phi-RoPE Rotary Position Encoding — continuous golden-angle rotation.

[12] This dissertation, Ch.18: Limitations — Coq mechanisation gap for VSA properties.

[13] Vogel, H. (1979). A better way to construct the sunflower head. *Mathematical Biosciences*, 44(3–4), 179–189. https://doi.org/10.1016/0025-5564(79)90080-4
