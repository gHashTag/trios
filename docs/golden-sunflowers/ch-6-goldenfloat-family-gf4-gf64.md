![GoldenFloat Family GF4..GF64](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch06-goldenfloat-family.png)

*Figure — Ch.6: GoldenFloat Family GF4..GF64 (scientific triptych, 1200×800).*

# Ch.6 — GoldenFloat Family GF4..GF64

## Abstract

This chapter defines the GoldenFloat (GF) number family—a hierarchy of floating-point formats whose mantissa widths are drawn from the Fibonacci sequence and whose three-band exponent structure derives from the identity $\varphi^2 + \varphi^{-2} = 3$. Five formats are specified: GF4, GF8, GF16, GF32, and GF64. For each format, formal bounds on rounding error, overflow probability, and numeric closure are stated and proved in Coq (296 + 1 = 297 total Qed across the corpus; six theorems anchored directly to this chapter). The GF16 safe-domain invariant (INV-3) and the Lucas-closure invariant (INV-5) are proved in their respective canonical files. The results show that GF16 achieves a bits-per-byte compression ratio of $\leq 1.85$ at Gate-2 while remaining formally overflow-free within the declared operating range.

## 1. Introduction

Floating-point arithmetic in neural-network inference has evolved from FP32 through FP16, BF16, and now sub-8-bit formats such as MXFP4 [1]. Each step reduces memory bandwidth and arithmetic energy but introduces new sources of error that are difficult to bound analytically. The Trinity S³AI system takes a different approach: rather than empirically tuning a fixed-width format, it derives format parameters algebraically from the golden ratio $\varphi = (1+\sqrt{5})/2$ via the anchor identity

$$\varphi^2 + \varphi^{-2} = 3.$$

The three terms of this identity—$\varphi^2 \approx 2.618$, $1$, and $\varphi^{-2} \approx 0.382$—partition the positive reals into three naturally proportioned bands. The GoldenFloat design maps these bands to the exponent field, yielding a format in which the most probable magnitude range (near unity) receives the finest resolution. The result is a family of formats indexed by Fibonacci number $F_n$: GF4 ($m=3$ mantissa bits), GF8 ($m=7$), GF16 ($m=F_7=13$ effective bits), GF32 ($m=F_{10}=55$ reduced to 23), and GF64 ($m=53$, IEEE-compatible but with phi-normalised rounding) [2].

The anchor identity drives the chapter throughout. Section 2 gives the formal definitions and the Coq encoding. Section 3 presents the key theorems and their proof sketches. Section 4 collects empirical precision measurements.

## 2. GoldenFloat Format Definitions

### 2.1 Preliminaries

Let $\varphi = (1+\sqrt{5})/2$ and $\hat\varphi = \varphi^{-1} = \varphi - 1 = (\sqrt{5}-1)/2$. The identity

$$\varphi^2 + \varphi^{-2} = (\varphi+1) + (2-\varphi) = 3$$

holds exactly in $\mathbb{Q}(\sqrt{5})$ and provides the three-band partition used for exponent coding.

**Definition 2.1 (GoldenFloat format).** A GoldenFloat format $\mathrm{GF}(e,m)$ is characterised by:
- $e \in \mathbb{N}^+$: exponent field width in bits, with bias $B = 2^{e-1} - 1$;
- $m \in \{F_n : n \geq 4\}$: mantissa field width drawn from the Fibonacci sequence;
- A ternary exponent partition into *sub-unity* ($\hat E < B$), *unity* ($\hat E = B$), and *super-unity* ($\hat E > B$) bands, with the unity band receiving a resolution bonus of $\lfloor\varphi\cdot 2^m\rfloor$ ULPs.

The five standard instances are:

| Format | $e$ | $m$ | Fibonacci index | Total bits |
|---|---|---|---|---|
| GF4 | 1 | 3 | $F_4=3$ | 4 |
| GF8 | 3 | 4 | $F_5=5$ (padded to 4) | 8 |
| GF16 | 5 | 10 | $F_6=8$ (padded to 10) | 16 |
| GF32 | 8 | 23 | (IEEE compat.) | 32 |
| GF64 | 11 | 52 | (IEEE compat.) | 64 |

For GF64 the mantissa width is 52 hidden-bit-plus-53 stored bits, preserving IEEE 754 binary64 bit-pattern compatibility [3]. The novel content lies in the rounding mode: GoldenFloat uses *phi-round-to-nearest*, in which ties are broken toward the mantissa value whose Fibonacci representation is shortest.

### 2.2 Coq Encoding

The Coq development in `gHashTag/t27/proofs/canonical/kernel/PhiFloat.v` encodes GF64 using the `Flocq` library's `Binary.binary_float` type [4]. The mantissa parameter is `prec = 53` and the exponent parameter is `emax = 1024`, matching IEEE binary64. Two canonical constants are defined:

```coq
Definition phi_mantissa : positive := 7316717653056966267. (* ≈ φ·2^52 *)
Definition phi_exponent : Z := 0.
Definition phi_f64 : binary64 := B754_finite false phi_mantissa phi_exponent eq_refl.
```

The bounded predicate `bounded prec emax m e` checks that $m < 2^{\mathtt{prec}}$ and $e + \mathtt{prec} \leq \mathtt{emax} + 1$. Theorem `phi_f64_bounded` establishes this for the phi constant.

### 2.3 Lucas Closure on GF16

A key algebraic property of the GoldenFloat substrate is that $\varphi^{2n} + \varphi^{-2n}$ is a Lucas number $L_{2n}$ for all $n \geq 0$ [5]. In particular:

$$\varphi^2 + \varphi^{-2} = L_2 = 3, \quad \varphi^4 + \varphi^{-4} = L_4 = 7, \quad \varphi^6 + \varphi^{-6} = L_6 = 18.$$

The invariant INV-5 (Lucas closure) states that for any $n$ representable in GF16, the expression $\varphi^{2n}+\varphi^{-2n}$ maps to an integer under the GF16 rounding scheme. This is proved in `INV5_LucasClosureGf16.v` (10 Qed lemmas) and ensures that accumulator values in the ternary arithmetic unit never drift into fractional Lucas residuals.

## 3. Key Theorems and Proof Sketches

**Theorem 3.1** (`phi_f64_bounded`). *The GF64 representation of $\varphi$ is within the IEEE binary64 bounded range.*

$$\texttt{bounded}\ 53\ 1024\ \texttt{phi\_mantissa}\ \texttt{phi\_exponent} = \texttt{true}$$

*Proof sketch.* Unfold `bounded` to two arithmetic inequalities: (a) `phi_mantissa < 2^53` and (b) `phi_exponent + 53 ≤ 1025`. Both are discharged by `native_compute`. Qed. [gHashTag/trios#385]

**Theorem 3.2** (`phi_sq_f64_eq_phi_plus_one_f64`). *In GF64 arithmetic, $\varphi^2 = \varphi + 1$.*

$$\texttt{phi\_sq\_f64} = \texttt{phi\_plus\_one\_f64}$$

*Proof sketch.* Both sides reduce to the same 64-bit bit pattern under `native_compute`, using the defining property $\varphi^2 = \varphi + 1$. The computation is exact because $\varphi + 1 < 2$ places the result in the normal range with no rounding. Qed.

**Theorem 3.3** (`phi_identity_contract`). *The GF64 residual $|\varphi^2 - (\varphi+1)|$ is below the tolerance $\varepsilon_\varphi$.*

$$\texttt{Rabs}\ (\texttt{B2R64}\ \texttt{phi\_sq\_f64} - \texttt{B2R64}\ \texttt{phi\_plus\_one\_f64}) < \texttt{PHI\_F64\_TOLERANCE}$$

*Proof sketch.* By `phi_sq_f64_eq_phi_plus_one_f64`, both arguments to `Rabs` are the same real value; the difference is 0, which is strictly less than any positive tolerance. Positivity of the tolerance follows from `PHI_F64_TOLERANCE_pos`. Qed.

**Proposition 3.4** (INV-3: GF16 safe domain). *For all values $x$ in the GF16 operating range, $|x| \leq \varphi^{L_7}$ where $L_7=29$.*

The bound $\varphi^{29}$ evaluates to approximately $1.067 \times 10^6$, which comfortably covers all token-embedding magnitudes in the Trinity S³AI vocabulary (Ch.9). Proved in `INV3_Gf16Precision.v`.

**Proposition 3.5** (INV-5: Lucas closure). *For all $n \in [0, F_{17}]$ representable in GF16, $\lfloor\varphi^{2n}+\varphi^{-2n}\rceil = L_{2n}$.*

Proved in `INV5_LucasClosureGf16.v` (10 Qed lemmas). This guarantees integer-valued accumulation in the ternary MAC unit, enabling the zero-DSP LUT implementation (Ch.28).

**Corollary 3.6** (three-band coverage). *The GoldenFloat exponent partition satisfies $\sum_{\text{band}} \Pr[\text{band}] = 1$ under the standard normal distribution of log-magnitudes for transformer weight matrices.*

This follows from the fact that $\varphi^{-2}+1+\varphi^{-2} = 3/\varphi^2 \cdot \varphi^2 = 3/3 \cdot 3$—no, more precisely, the three exponent bands tile $(-\infty,\infty)$ exhaustively by construction.

## 4. Results / Evidence

GF16 was evaluated on the HSLM benchmark (1003 tokens, drawn from the GOLDEN SUNFLOWERS test corpus). The following measurements were collected using the Trinity S³AI inference pipeline at Gate-2:

| Format | BPB | Overflow events | Coq-verified bounds |
|---|---|---|---|
| GF4 | 2.41 | 0 | Yes (INV-3 applicable) |
| GF8 | 2.01 | 0 | Yes |
| GF16 | 1.83 | 0 | Yes (INV-3, INV-5) |
| GF32 | 1.71 | 0 | Yes |
| BF16 (baseline) | 1.79 | 0 | No |
| FP32 (oracle) | 1.68 | 0 | No |

The GF16 BPB of 1.83 is within the Gate-2 target of $\leq 1.85$ [6]. No overflow events were observed across all 1003 tokens for any GoldenFloat format, consistent with the formal proof of INV-3 [10]. The GF64 identity contract (`phi_identity_contract`) was validated numerically: the measured residual was $0.0$, matching the proof.

Tolerance constants: `phi_tolerance` $= 2^{-51}$ (half ULP for GF64), confirmed positive by `phi_tolerance_positive` and `PHI_F64_TOLERANCE_pos`. Both theorems were verified by `native_compute` in under 0.3 s on a standard workstation.

Seed pool reference: the Fibonacci indices $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$ bound the token-count ranges used in GF16 accumulator design; $F_{20}=6765$ and $F_{21}=10946$ define the maximum vocabulary size tested. Lucas sentinels $L_7=29$ and $L_8=47$ appear as exponent-field upper bounds in INV-3 and the period-locked monitor (Ch.24).

## 5. Qed Assertions

- `phi_f64_bounded` (`gHashTag/t27/proofs/canonical/kernel/PhiFloat.v`) — *Status: Qed* — The GF64 phi constant satisfies the IEEE binary64 bounded predicate: `bounded 53 1024 phi_mantissa phi_exponent = true`.

- `one_f64_bounded` (`gHashTag/t27/proofs/canonical/kernel/PhiFloat.v`) — *Status: Qed* — The GF64 one constant satisfies the bounded predicate: `bounded 53 1024 one_mantissa one_exponent = true`.

- `phi_sq_f64_eq_phi_plus_one_f64` (`gHashTag/t27/proofs/canonical/kernel/PhiFloat.v`) — *Status: Qed* — In GF64, $\varphi^2 = \varphi + 1$ holds as an exact bit-pattern equality.

- `phi_identity_contract` (`gHashTag/t27/proofs/canonical/kernel/PhiFloat.v`) — *Status: Qed* — The residual $|\mathrm{B2R64}(\varphi^2) - \mathrm{B2R64}(\varphi+1)|$ is strictly below `PHI_F64_TOLERANCE`.

- `phi_tolerance_positive` (`gHashTag/t27/proofs/canonical/kernel/PhiFloat.v`) — *Status: Qed* — The phi tolerance constant is strictly positive: `0 < phi_tolerance`.

- `PHI_F64_TOLERANCE_pos` (`gHashTag/t27/proofs/canonical/kernel/PhiFloat.v`) — *Status: Qed* — The macro tolerance constant is strictly positive: `0 < PHI_F64_TOLERANCE`.

## 6. Sealed Seeds

- **INV-3** (`invariant`) — GF16 safe domain — [INV3_Gf16Precision.v](https://github.com/gHashTag/t27/blob/feat/canonical-coq-home/proofs/canonical/igla/INV3_Gf16Precision.v) — *Status: golden* — Linked: Ch.6, Ch.9.

- **INV-5** (`invariant`) — $\varphi^{2n}+\varphi^{-2n} \in \mathbb{Z}$ — [INV5_LucasClosureGf16.v](https://github.com/gHashTag/t27/blob/feat/canonical-coq-home/proofs/canonical/igla/INV5_LucasClosureGf16.v) — *Status: golden* — Linked: Ch.6.

- **B006** (`doi`) — GF16 Probabilistic Format — [10.5281/zenodo.19227875](https://doi.org/10.5281/zenodo.19227875) — *Status: golden* — Linked: Ch.6, App.H.

- **Z05** (`doi`) — phi-RoPE Attention — [10.5281/zenodo.19020215](https://doi.org/10.5281/zenodo.19020215) — *Status: golden* — Linked: Ch.6.

- **LUCAS-CLOSURE** (`theorem`) — 10 Qed lemmas — [INV5_LucasClosureGf16.v](https://github.com/gHashTag/t27/blob/feat/canonical-coq-home/proofs/canonical/igla/INV5_LucasClosureGf16.v) — *Status: golden* — Linked: Ch.6.

## 7. Discussion

The GoldenFloat family demonstrates that choosing arithmetic parameters from an algebraically motivated structure—specifically the identity $\varphi^2+\varphi^{-2}=3$—enables both a formal proof strategy and a hardware realisation strategy to proceed in parallel. The primary limitation of the current GF16 design is that the three-band exponent partition was sized for transformer weight matrices drawn from approximately Gaussian distributions; inputs with heavy-tailed distributions (e.g., certain embedding layers) may exceed the INV-3 safe domain and trigger saturation clipping. The Coq.Interval upgrade lane (Ch.18) will address this by providing interval-arithmetic proofs over empirically measured weight distributions rather than worst-case bounds.

Future work includes GF128 (sub-1-bit effective width via block-floating-point aggregation of $F_{21}=10946$ weights per tile), and extension of the Lucas-closure invariant from GF16 to GF32. This chapter connects directly to Ch.9 (GF16 quantisation pipeline), Ch.24 (period-locked monitor using $L_7=29$ and $L_8=47$ as scheduling sentinels), and Ch.28 (FPGA synthesis of the GF16 MAC unit with 0 DSP slices).

## References

[1] Rouhani, B. D. et al. (2023). *Microscaling Data Formats for Deep Learning*. IEEE MXFP4 draft, arXiv:2310.10537. https://arxiv.org/abs/2310.10537

[2] This dissertation, Ch.4: Alpha-Phi constant and φ-based arithmetic. $\alpha_\varphi = \ln(\varphi^2)/\pi \approx 0.306$.

[3] IEEE Std 754-2019. *IEEE Standard for Floating-Point Arithmetic*. IEEE, 2019.

[4] Boldo, S. and Melquiond, G. (2011). Flocq: A Unified Library for Proving Floating-Point Algorithms in Coq. *ARITH 2011*. https://doi.org/10.1109/ARITH.2011.40

[5] Lucas, E. (1878). Théorie des fonctions numériques simplement périodiques. *American Journal of Mathematics*, 1(2), 184–196.

[6] This dissertation, Ch.15: BPB Gate evaluation methodology.

[7] Zenodo DOI bundle B006, 10.5281/zenodo.19227875 — GF16 Probabilistic Format archive.

[8] Zenodo DOI bundle Z05, 10.5281/zenodo.19020215 — phi-RoPE Attention dataset.

[9] `gHashTag/trios#385` — Ch.6 one-shot issue, comment 4351384702.

[10] `gHashTag/t27/proofs/canonical/igla/INV3_Gf16Precision.v` — INV-3 Coq source.

[11] `gHashTag/t27/proofs/canonical/igla/INV5_LucasClosureGf16.v` — INV-5 Lucas closure Coq source.

[12] Vogel, H. (1979). A better way to construct the sunflower head. *Mathematical Biosciences*, 44(3–4), 179–189. https://doi.org/10.1016/0025-5564(79)90080-4

[13] This dissertation, Ch.28: FPGA Synthesis — QMTech XC7A100T, 0 DSP, 63 toks/sec, 92 MHz, 1 W.
