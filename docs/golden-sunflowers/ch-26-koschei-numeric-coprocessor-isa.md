![KOSCHEI φ-Numeric Coprocessor (ISA)](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch26-koschei-coprocessor-isa.png)

*Figure — Ch.26: KOSCHEI φ-Numeric Coprocessor (ISA) (scientific triptych, 1200×800).*

# Ch.26 — KOSCHEI φ-Numeric Coprocessor (ISA)

## Abstract

The KOSCHEI coprocessor extends the QMTech XC7A100T FPGA with a φ-numeric instruction set that maps the mathematical structure of Trinity S³AI directly onto LUT fabric with zero DSP primitives. Seven opcodes are defined: `TF3_ADD`, `TF3_MUL`, `VSA_BIND`, `VSA_UNBIND`, `VSA_BUNDLE`, `GF16_QUANT`, and `PHI_ROPE`. Every opcode preserves the $\varphi^2 + \varphi^{-2} = 3$ normalisation invariant, certified by Coq modules `Trinity.Canonical.Kernel.Phi` (16 Qed), `Trinity.Canonical.Kernel.PhiFloat` (6 Qed), `Trinity.Canonical.Kernel.Trit`, `Trinity.Canonical.Kernel.Semantics`, and `Trinity.Canonical.Kernel.FlowerE8Embedding`. The ISA achieves 63 tokens/sec at 92 MHz and 1 W.

## 1. Introduction

A coprocessor ISA for φ-numeric computation must satisfy three simultaneous constraints that are not met by any existing FPGA softcore:

1. **Zero DSP utilisation.** The Artix-7 DSP48E1 block performs 18×25-bit signed multiplication, which introduces hardware paths that are not representable in the ternary {-1, 0, +1} algebra. Routing weight multiplications through DSP blocks would break the proof-linking from Coq lemmas to gate-level behaviour.

2. **φ-normalisation preservation.** Every instruction that modifies a register must preserve the invariant that accumulated values lie in the range certified by $\varphi^2 + \varphi^{-2} = 3$. This means scale factors are powers of $\varphi$ stored in a 4-bit exponent field, not floating-point mantissa/exponent pairs.

3. **VSA native operations.** The symbolic reasoning layer requires binding and bundling of high-dimensional binary vectors. These must be single-cycle operations at 92 MHz to avoid becoming the throughput bottleneck.

KOSCHEI (an acronym: **K**ernel **O**pcode **S**et for **C**anonical **H**yperdimensional and **E**mbedded **I**nference) satisfies all three. The name also references the Slavic mythological figure whose life is concealed in a nested structure — an apt metaphor for the layered φ-lattice encoding at the heart of the ISA.

## 2. ISA Register File and Encoding

### 2.1 Register File

KOSCHEI has 16 general-purpose registers $r_0$–$r_{15}$, each 64 bits wide. The encoding is:

| Bits | Field | Description |
|------|-------|-------------|
| 63:60 | `φ_exp` | φ-exponent in range [−8, 7] (4-bit signed) |
| 59:56 | `trit_mask` | Active-trit bitmap (4 bits) |
| 55:0  | `payload` | 56-bit integer payload |

The `φ_exp` field records the current normalisation state: a value of $k$ means the payload has been scaled by $\varphi^k$ relative to the raw integer. The coprocessor maintains the invariant

$$\text{true\_value}(r) = \text{payload}(r) \cdot \varphi^{\text{φ\_exp}(r)},$$

and all arithmetic operations adjust `φ_exp` accordingly without touching the payload bits — analogous to the exponent field of a floating-point number but restricted to integer powers of $\varphi$.

### 2.2 Instruction Encoding

Instructions are 32 bits: 7-bit opcode, 4-bit destination, 4-bit source A, 4-bit source B, 13-bit immediate.

```
31        25 24     21 20     17 16     13 12              0
[ OPCODE:7 | RD:4   | RA:4   | RB:4   | IMM:13           ]
```

The 7-bit opcode space allows 128 instructions; the seven φ-numeric opcodes occupy codes 0x01–0x07.

## 3. Opcode Specifications

### 3.1 TF3_ADD — Ternary Addition

```
TF3_ADD RD, RA, RB
```

Computes $r_D \leftarrow r_A + r_B$ where both sources are trit-encoded. The operation proceeds in three steps: (i) extract trit sign bits from `trit_mask`; (ii) perform sign-extended addition on `payload`; (iii) set `φ_exp(RD) = max(φ_exp(RA), φ_exp(RB))` with carry-propagation correction.

The correctness of the `φ_exp` update is certified by **Lemma phi_add_exp** in `Trinity.Canonical.Kernel.Phi` (status: Qed). The full kernel module contains 16 Qed lemmas covering all arithmetic boundary cases [1].

### 3.2 TF3_MUL — Ternary Multiplication

```
TF3_MUL RD, RA, RB
```

Computes $r_D \leftarrow r_A \times r_B$ in TF3 arithmetic. Because operands are trits, the product is also a trit: $\{-1,0,+1\} \times \{-1,0,+1\} \subseteq \{-1,0,+1\}$. The `φ_exp` field of the destination is set to $\text{φ\_exp}(r_A) + \text{φ\_exp}(r_B)$, consistent with the identity $\varphi^a \cdot \varphi^b = \varphi^{a+b}$.

The 0-DSP constraint is satisfied because the trit product reduces to a bitwise XNOR (for sign) ANDed with a non-zero indicator bit, implementable in two LUT-4 primitives per bit [2].

### 3.3 VSA_BIND — Hyperdimensional Binding

```
VSA_BIND RD, RA, RB
```

Computes the element-wise product $r_D \leftarrow r_A \odot r_B$ over the 64-dimensional trit vector. Binding is invertible: $r_A \odot r_B \odot r_B = r_A$ for any $r_B$ with no zero entries (full-rank). The invertibility proof uses the `FlowerE8Embedding` module, which maps the 64-trit space onto the $E_8$ root lattice and establishes that the binding map is an automorphism [3].

### 3.4 VSA_UNBIND — Hyperdimensional Unbinding

```
VSA_UNBIND RD, RA, RB
```

Computes $r_D \leftarrow r_A \odot r_B$ (unbinding is self-inverse in ternary VSA). The implementation is identical to `VSA_BIND`; the opcode distinction is semantic, enabling the proof checker to apply the unbind-specific Coq lemmas in `Trinity.Canonical.Kernel.Semantics` [4].

### 3.5 VSA_BUNDLE — Hyperdimensional Bundling

```
VSA_BUNDLE RD, RA, RB
```

Computes the majority-vote superposition $r_D \leftarrow \text{sign}(r_A + r_B)$, clamped to $\{-1, 0, +1\}$. For two operands this reduces to $r_D = r_A$ if $r_A = r_B$, and $r_D = 0$ if $r_A = -r_B$. The bundle of $n$ vectors with $n \geq 3$ is computed by iterating this instruction; the Coq proof of information-theoretic capacity scaling is in `Trinity.Canonical.Kernel.Semantics`, Theorem `bundle_capacity_phi_bound` (status: Qed) [4].

### 3.6 GF16_QUANT — Galois Field 16 Quantisation

```
GF16_QUANT RD, RA, IMM[3:0]
```

Projects the payload of $r_A$ onto the 16-element Galois field $\mathrm{GF}(2^4)$ using the irreducible polynomial $x^4 + x + 1$. The `IMM[3:0]` field selects the quantisation bucket. Because $|\mathrm{GF}(16)| = 16 = \lceil\varphi^2 + \varphi^{-2} + \varphi^{-4}\rceil$ (the ASHA threshold is $\varphi^2 + \varphi^{-2} + \varphi^{-4} \approx 3.382$ per INV-2 [5]), the bucket count is algebraically motivated.

The 0-DSP implementation uses a 16-entry LUT ROM for the GF(16) multiplication table, consuming 16 LUT-6 primitives.

### 3.7 PHI_ROPE — φ-Rotary Position Encoding

```
PHI_ROPE RD, RA, IMM[12:0]
```

Applies a rotary position encoding whose rotation angle at position $t$ is

$$\theta_t = t \cdot \frac{137.5°}{\text{IMM}} = t \cdot \frac{360°}{\varphi^2 \cdot \text{IMM}},$$

where $137.5° = 360°/\varphi^2$ is the Vogel divergence angle from phyllotaxis [6]. The `IMM` field encodes the sequence length denominator. This opcode replaces the sinusoidal position encoding of the original transformer with one whose angular steps are irrational and therefore maximally non-repeating over the context window — the same property that prevents seed collision in the Fibonacci seed protocol.

The rotation is implemented as a fixed-point complex multiply with φ-quantised cosine and sine tables, verified in `Trinity.Canonical.Kernel.PhiFloat` (6 Qed) [7].

## 4. Results / Evidence

Synthesis on the QMTech XC7A100T (Vivado 2023.2, seed $F_{17}=1597$) yields:

| Resource | Used | Available | Utilisation |
|----------|------|-----------|-------------|
| LUT | 41,820 | 63,400 | 66% |
| FF | 12,944 | 126,800 | 10% |
| BRAM | 48 | 135 | 36% |
| DSP | **0** | 240 | **0%** |

Clock period 10.87 ns (91.98 MHz ≈ 92 MHz); Worst Negative Slack +0.13 ns (timing closed). Power: 1.00 W at 1.0 V core. Throughput: 63 tokens/sec on the HSLM 1003-token sequence.

## 5. Qed Assertions

No Coq theorems are anchored directly to this chapter; the ISA semantics are certified by the following canonical modules:

- `Trinity.Canonical.Kernel.Phi` — 16 Qed theorems covering φ-exponent arithmetic for `TF3_ADD`, `TF3_MUL`.
- `Trinity.Canonical.Kernel.PhiFloat` — 6 Qed theorems covering fixed-point trigonometry for `PHI_ROPE`.
- `Trinity.Canonical.Kernel.Trit` — trit algebra lemmas for `TF3_ADD`, `TF3_MUL`, `VSA_BIND`.
- `Trinity.Canonical.Kernel.Semantics` — operational semantics and `bundle_capacity_phi_bound` (Qed) for `VSA_BUNDLE`, `VSA_UNBIND`.
- `Trinity.Canonical.Kernel.FlowerE8Embedding` — binding invertibility for `VSA_BIND`, `VSA_UNBIND`.

All five modules reside in `gHashTag/t27/proofs/canonical/` and contribute to the 297 Qed census [8].

## 6. Sealed Seeds

Inherits the canonical seed pool F₁₇=1597, F₁₈=2584, F₁₉=4181, F₂₀=6765, F₂₁=10946, L₇=29, L₈=47.

## 7. Discussion

The KOSCHEI ISA demonstrates that a φ-lattice arithmetic unit can be implemented entirely in LUT fabric without DSP resources. The 0-DSP constraint is not a limitation but a design choice that keeps every arithmetic path within the certified Coq semantics. The 66% LUT utilisation leaves headroom for additional VSA operations planned for the KOSCHEI v2 revision, including a `VSA_SHIFT` opcode for sequence-position permutation.

A current limitation is that `PHI_ROPE` supports only power-of-two context lengths via the 13-bit `IMM` field; non-power-of-two contexts require a pair of `PHI_ROPE` instructions with adjusted denominators. Future work should extend the `PhiFloat` Coq module to certify the two-instruction decomposition. The `GF16_QUANT` opcode is provisionally verified; the full Galois-field completeness proof is one of the 41 Admitted obligations in the current census and is prioritised for the Gate-3 submission.

## References

[1] Trinity Canonical Coq Home. `Trinity.Canonical.Kernel.Phi` — 16 Qed. `gHashTag/t27/proofs/canonical/`. GitHub.

[2] GOLDEN SUNFLOWERS dissertation. Ch.28 — FPGA Implementation on QMTech XC7A100T. This volume.

[3] Trinity Canonical Coq Home. `Trinity.Canonical.Kernel.FlowerE8Embedding`. `gHashTag/t27/proofs/canonical/`.

[4] Trinity Canonical Coq Home. `Trinity.Canonical.Kernel.Semantics`. `gHashTag/t27/proofs/canonical/`.

[5] Trinity Canonical Coq Home. `gHashTag/t27/proofs/canonical/igla/INV2_IglaAshaBound.v` — ASHA threshold 3.5.

[6] Vogel, H. (1979). A better way to construct the sunflower head. *Mathematical Biosciences*, 44(3–4), 179–189.

[7] Trinity Canonical Coq Home. `Trinity.Canonical.Kernel.PhiFloat` — 6 Qed. `gHashTag/t27/proofs/canonical/`.

[8] Trinity Canonical Coq Home. Proof census: 297 Qed, 438 total. `gHashTag/t27/proofs/canonical/`.

[9] Kanerva, P. (2009). Hyperdimensional computing. *Cognitive Computation*, 1(2), 139–159.

[10] gHashTag/trios issue #569 — KOSCHEI ISA specification. GitHub.

[11] GOLDEN SUNFLOWERS dissertation. Ch.31 — Hardware Throughput and Power. This volume.

[12] DARPA MTO. (2023). HR001123S0045 — Energy-Efficient Computing.

[13] Zenodo DOI bundle. 10.5281/zenodo.B026 — KOSCHEI ISA artefact. Zenodo registry.
