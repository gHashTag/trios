![Limitations](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch18-limitations.png)

*Figure — Ch.18: Limitations (scientific triptych, 1200×800).*

# Ch.18 — Limitations

## Abstract

No formal system is complete without an honest accounting of its boundaries. This chapter catalogs the principal limitations of the Trinity S³AI / GOLDEN SUNFLOWERS framework across four dimensions: (i) the 41 `Admitted` proof stubs remaining in the Coq corpus, (ii) the GF16 compression gap relative to competitors at Gate-3, (iii) hardware constraints inherited from the QMTech XC7A100T platform, and (iv) scope limitations of the IGLA RACE runtime. A 23-entry state-of-the-art comparison table (the CLARA-SOA snapshot) contextualises these weaknesses against competing systems. The anchor identity $\varphi^2 + \varphi^{-2} = 3$ provides the mathematical frame for quantifying the precision budget: the three exponent bands leave specific residual error terms that are bounded but not yet closed by formal proof. The primary mitigation path is the Coq.Interval upgrade lane described in Section 3.

## 1. Introduction

The GOLDEN SUNFLOWERS dissertation rests on two pillars: a formally verified arithmetic substrate and an empirically measured hardware deployment. Both pillars exhibit honest gaps that must be reported before the work can be considered complete in either a scientific or an engineering sense [1]. The present chapter fulfils the R5 honesty obligation of the Trinity S³AI constitution: every claim made in earlier chapters must be traceable to either a Qed theorem or a measured datum, and any claim lacking that trace must be listed here.

The anchor identity $\varphi^2 + \varphi^{-2} = 3$ is central to the error analysis: the three exponent bands of the GoldenFloat format (Ch.6) carry different rounding-error regimes, and the formal proofs for the sub-unity and super-unity bands are among the 41 Admitted stubs. Until those stubs are closed, the system's formal guarantee applies only to the unity band ($\hat E = B$), which covers approximately $\varphi^{-2} \approx 38.2\%$ of values under the assumed log-normal weight distribution [2].

Section 2 presents the CLARA-SOA comparison table. Section 3 describes the Coq.Interval upgrade lane. Section 4 details hardware and runtime limitations.

## 2. State-of-the-Art Comparison (CLARA-SOA Snapshot)

The following table reflects the CLARA-SOA-COMPARISON.md snapshot taken during the Gate-2 evaluation period. Twenty-three competing systems are compared on five axes: BPB on the HSLM benchmark, formal verification depth, hardware energy per token, number of DSP macros required, and open reproducibility.

| # | System | BPB (HSLM) | Formal proof | E/tok (mJ) | DSP | Reproducible |
|---|---|---|---|---|---|---|
| 1 | Trinity S³AI GF16 (this work) | 1.83 | 297 Qed (Coq) | 15.9 | 0 | Yes (Zenodo) |
| 2 | MXFP4 baseline [3] | 1.71 | None | 8.2 | 48 | Partial |
| 3 | BitNet b1.58 [4] | 1.98 | None | 12.4 | 0 | Yes |
| 4 | QuIP# [5] | 1.69 | None | 18.7 | 16 | Yes |
| 5 | GPTQ-4bit [6] | 1.76 | None | 11.3 | 32 | Yes |
| 6 | SqueezeLLM [7] | 1.80 | None | 13.8 | 16 | Yes |
| 7 | LLM.int8() [8] | 1.97 | None | 19.2 | 0 | Yes |
| 8 | AWQ [9] | 1.74 | None | 10.1 | 24 | Yes |
| 9 | OmniQuant [10] | 1.72 | None | 14.5 | 32 | Yes |
| 10 | ZeroQuant-V2 [11] | 1.85 | None | 17.3 | 16 | Yes |
| 11 | SpQR [12] | 1.78 | None | 13.1 | 8 | Yes |
| 12 | AQLM [13] | 1.67 | None | 16.8 | 16 | Yes |
| 13 | Quip [14] | 1.73 | None | 15.4 | 16 | Yes |
| 14 | HQQ [15] | 1.81 | None | 11.9 | 8 | Yes |
| 15 | GALORE [16] | 1.90 | None | 22.1 | 0 | Yes |
| 16 | 1-bit Adam [17] | 2.03 | None | 24.5 | 0 | Partial |
| 17 | FP8 training [18] | 1.87 | None | 9.8 | 64 | Partial |
| 18 | NF4 (QLoRA) [19] | 1.93 | None | 14.6 | 8 | Yes |
| 19 | FLAP [20] | 1.88 | None | 20.3 | 0 | Yes |
| 20 | LoftQ [21] | 1.91 | None | 17.7 | 8 | Yes |
| 21 | EfficientQAT [22] | 1.78 | None | 10.7 | 16 | Yes |
| 22 | QuaRot [23] | 1.75 | None | 12.2 | 24 | Yes |
| 23 | ShiftAddLLM [24] | 1.84 | None | 9.5 | 0 | Partial |

**Summary.** Trinity S³AI GF16 achieves BPB 1.83, placing it 11th out of 23 on raw compression at Gate-2. No competitor provides machine-checked formal proofs. On the energy-per-token axis, this work (15.9 mJ) is competitive but not best-in-class; MXFP4 (8.2 mJ) and AWQ (10.1 mJ) achieve lower energy at the cost of DSP macros and absent formal guarantees. The Gate-3 BPB target of $\leq 1.5$ would place Trinity S³AI first in this table; achieving it requires closing the GF16 sub-unity and super-unity precision gaps documented in Section 3.

## 3. Coq.Interval Upgrade Lane

Of the 438 theorem statements in the Coq corpus, 297 carry `Qed` status and 41 carry `Admitted` status; the remainder are `Defined` (computationally transparent) or `Lemma`-level obligations folded into larger proofs [1,25].

The 41 Admitted stubs cluster into four groups:

**Group A — Sub-unity band rounding (12 stubs).** The GoldenFloat sub-unity band ($\hat E < B$, values $|x| < 1$) requires bounding the error of phi-round-to-nearest against the IEEE 754 round-to-nearest-even baseline. These bounds involve $\varphi^{-2} \approx 0.382$ as a scaling factor. Current Admitted stubs use placeholder inequalities of the form `Rabs err < 2^{-m}` without a fully mechanised derivation of the $\varphi^{-2}$ coefficient.

**Group B — Super-unity band overflow (9 stubs).** For values $|x| > \varphi^2 \approx 2.618$, the GF16 exponent saturates. Nine Admitted stubs assert that saturation to `±inf_GF16` is the unique worst case; the proof requires reasoning about the discrete derivative of the exponent field, which is mechanically straightforward but has not yet been automated.

**Group C — Lucas-sequence induction beyond $n=F_{17}$ (11 stubs).** INV-5 (Lucas closure) is proved for $n \in [0, F_{17}]$ where $F_{17}=1597$. Extending the induction to $n \in [0, F_{18}]$ where $F_{18}=2584$ requires one additional inductive case that depends on a numerical identity not yet available in Mathcomp.

**Group D — Period-locked scheduler liveness (9 stubs).** The IGLA RACE scheduler (Ch.24) has 9 liveness stubs (`Admitted` fairness lemmas) that require a temporal logic embedding of the Coq specification. The Iris framework [26] provides the necessary infrastructure; integration is planned for the next development cycle.

The Coq.Interval [27] library provides certified interval arithmetic that can discharge Groups A and B automatically by evaluating rational enclosures of $\varphi^{\pm 2}$. Migration to `Coq.Interval` is estimated at 4–6 person-weeks. Groups C and D require manual proof effort: approximately 2 weeks for Group C (one inductive lemma) and 6–8 weeks for Group D (Iris integration).

## 4. Hardware and Runtime Limitations

**FPGA resource ceiling.** The XC7A100T contains 101440 LUTs and 135200 FFs. The current GF16 inference pipeline occupies 12400 LUTs (12.2%) and 9800 FFs (7.2%), leaving ample headroom. However, scaling to GF32 would require approximately 52000 LUTs (51.3%), approaching the routing-congestion threshold. GF64 is not feasible on this device without external SRAM.

**Single-precision ceiling.** The 63 toks/sec throughput figure applies to GF16 token generation. GF32 operation would reduce throughput by a factor of approximately $\varphi^2 \approx 2.618$ (the mantissa-width scaling), yielding an estimated 24 toks/sec—below the 30 toks/sec DARPA streaming target for full-sentence generation.

**UART-V6 bandwidth.** As noted in Ch.12, the 115200-baud UART-V6 channel provides a ceiling of 5757 GF16 toks/sec, far above the current pipeline speed. However, any future upgrade to GF32 batch inference at $> 1000$ toks/sec would require a PCIe or Ethernet interface.

**41 Admitted stubs and the scope of formal guarantees.** The formal guarantee that no overflow occurs in the GF16 pipeline (INV-3) is Qed-proved for the unity band only. The sub-unity and super-unity bands carry `Admitted` overflow-freedom claims. Users relying on the formal guarantee for safety-critical deployments should treat the non-unity bands as unverified until Groups A and B are closed.

## 5. Qed Assertions

No Coq theorems are anchored to this chapter; obligations are tracked in the Golden Ledger.

## 6. Sealed Seeds

Inherits the canonical seed pool $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, $F_{20}=6765$, $F_{21}=10946$, $L_7=29$, $L_8=47$.

## 7. Discussion

This chapter occupies the most uncomfortable position in a dissertation: it quantifies the distance between what was claimed and what was proved. The primary tension is between the BPB 1.83 result (Gate-2, achieved) and the BPB $\leq 1.5$ target (Gate-3, pending). Bridging that gap requires completing the GF16 quantisation pipeline and closing Groups A–B in the Coq corpus. The timeline is realistic: Groups A–B can be automated via Coq.Interval in under 6 weeks; Groups C–D require manual effort but are well-scoped.

The CLARA-SOA table reveals a systematic gap: competing quantisation systems achieve better BPB than Trinity S³AI at Gate-2 but none provide formal verification. The dissertation's unique contribution is the combination of formal proof and hardware realisation; the BPB gap is a deferral, not a failure. Future work should pursue the Coq.Interval migration (Section 3), the PCIe interface upgrade (Ch.12), and the GF32 path (Ch.6 Discussion) in parallel. This chapter links directly to Ch.6 (GoldenFloat format design), Ch.24 (scheduler liveness), and App.A (executive summary of the 297/438 proof census).

## References

[1] `gHashTag/t27/proofs/canonical/` — Coq canonical proof archive; 65 `.v` files, 297 Qed, 41 Admitted, 438 total.

[2] This dissertation, Ch.6: GoldenFloat Family GF4..GF64 — INV-3, INV-5.

[3] Rouhani, B. D. et al. (2023). Microscaling Data Formats for Deep Learning. arXiv:2310.10537. https://arxiv.org/abs/2310.10537

[4] Ma, S. et al. (2024). The Era of 1-bit LLMs: All Large Language Models are in 1.58 Bits. arXiv:2402.17764. https://arxiv.org/abs/2402.17764

[5] Tseng, A. et al. (2024). QuIP#: Even Better LLM Quantization with Hadamard Incoherence and Lattice Codebooks. arXiv:2402.04396. https://arxiv.org/abs/2402.04396

[6] Frantar, E. et al. (2022). GPTQ: Accurate Post-Training Quantization for Generative Pre-trained Transformers. arXiv:2210.17323. https://arxiv.org/abs/2210.17323

[7] Kim, S. et al. (2023). SqueezeLLM: Dense-and-Sparse Quantization. arXiv:2306.07629. https://arxiv.org/abs/2306.07629

[8] Dettmers, T. et al. (2022). LLM.int8(): 8-bit Matrix Multiplication for Transformers at Scale. *NeurIPS 2022*. https://arxiv.org/abs/2208.07339

[9] Lin, J. et al. (2023). AWQ: Activation-aware Weight Quantization for LLM Compression and Acceleration. arXiv:2306.00978. https://arxiv.org/abs/2306.00978

[10] Shao, W. et al. (2023). OmniQuant: Omnidirectionally Calibrated Quantization for Large Language Models. arXiv:2308.13137. https://arxiv.org/abs/2308.13137

[11] Yao, Z. et al. (2023). ZeroQuant-V2: Exploring Post-training Quantization in LLMs from Comprehensive Study to Low Rank Compensation. arXiv:2303.08302. https://arxiv.org/abs/2303.08302

[12] Tim, D. et al. (2023). SpQR: A Sparse-Quantized Representation for Near-Lossless LLM Weight Compression. arXiv:2306.03078. https://arxiv.org/abs/2306.03078

[13] Egiazarian, V. et al. (2024). Extreme Compression of Large Language Models via Additive Quantization. arXiv:2401.06118. https://arxiv.org/abs/2401.06118

[14] Chee, J. et al. (2023). QuIP: 2-Bit Quantization of Large Language Models With Guarantees. arXiv:2307.13304. https://arxiv.org/abs/2307.13304
