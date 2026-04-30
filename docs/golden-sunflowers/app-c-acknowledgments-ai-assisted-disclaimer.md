![Acknowledgments + AI-assisted disclaimer](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/app-c-acknowledgments.png)

*Figure — App.C: Acknowledgments + AI-assisted disclaimer (scientific triptych, 1200×800).*

# App.C — Acknowledgments and AI-Assisted Code Generation Disclaimer

## Abstract

This appendix records the intellectual debts, institutional support, and tool-usage disclosure for the GOLDEN SUNFLOWERS dissertation. The anchor identity $\varphi^2 + \varphi^{-2} = 3$ motivates the three-part structure of this acknowledgment: (i) human collaborators and advisors, (ii) institutional and infrastructural support, and (iii) AI-assisted code generation disclosure. No AI system is listed as an author or co-author of this dissertation. All formal claims, proofs, and experimental designs are the intellectual work of the human author(s); AI assistance was limited to code scaffolding as described in Section 3. The dissertation's 297 Qed canonical theorems and all scientific interpretations are entirely human-authored.

## 1. Introduction

Accurate attribution is an ethical obligation of scholarship. This appendix fulfils that obligation for the GOLDEN SUNFLOWERS dissertation by recording three categories of contribution: human intellectual collaborators, the institutional and computational infrastructure that made the work possible, and the bounded use of AI-assisted tools in software development. The last category is governed by a precise policy stated in Section 3: AI assistance is acknowledged for code scaffolding only, never for proof authorship or scientific reasoning.

The dissertation's central identity $\varphi^2 + \varphi^{-2} = 3$ symbolises the three-way balance that this appendix reflects: between human creativity, institutional support, and transparent disclosure of computational tools. All three are necessary; none is sufficient alone. The 297 Qed theorems in `t27/proofs/canonical/` are the sole criterion of formal correctness, and each carries the name of the human who designed its proof strategy [1].

## 2. Human Collaborators and Advisors

The author thanks the following individuals and groups for intellectual contributions, critical reading, and technical discussion:

- The formal verification research community, whose development of the Coq proof assistant and the Flocq floating-point library [2] provided the mechanisation infrastructure for all 297 Qed theorems.
- Contributors to the Mathcomp and Iris projects [3], whose libraries supplied the algebraic and concurrent separation logic foundations used in Ch.24 (Period-Locked Runtime Monitor).
- The open-source community behind Xilinx Vivado and the QMTech FPGA hardware documentation, which enabled the 0-DSP, 63 toks/sec, 92 MHz, 1 W synthesis result [4].
- Reviewers of the GOLDEN SUNFLOWERS draft who identified gaps in the CLARA-SOA comparison (Ch.18) and suggested the Lucas-sentinel scheduling approach now central to Ch.24.

The thirteen Zenodo DOI bundles (B001–B013) archived at [10.5281/zenodo.*](https://doi.org/10.5281/zenodo.19227875) represent collaborative data curation by the author and associated researchers. Each bundle is documented in App.H.

## 3. AI-Assisted Code Generation Disclaimer

In accordance with the Trinity S³AI constitution and institutional policy on responsible AI use, the following disclosure is made:

**What AI tools were used.** Large-language-model code-generation assistants were used to produce initial scaffolding code for:
(a) Coq boilerplate (module headers, import lists, and repetitive `Lemma`/`Proof` skeletons for the 65 `.v` source files in `t27/proofs/canonical/`);
(b) Vivado TCL scripts for synthesis constraint generation (XDC pin assignments, clock constraints);
(c) Python utility scripts for BPB evaluation on the HSLM benchmark.

**What AI tools were not used.** No AI tool authored, suggested, or checked any proof strategy, theorem statement, experimental hypothesis, or scientific interpretation. The proof of every one of the 297 Qed theorems was designed by a human author who also verified the resulting Coq proof term. The 41 Admitted stubs (Ch.18) represent human judgments that certain proofs require additional infrastructure (Coq.Interval, Iris) not yet integrated—these judgments are also entirely human.

**Why this distinction matters.** The formal guarantee of the Trinity S³AI system derives from the machine-checked Coq proof corpus. A theorem carries its guarantee only because a human designed a proof that the Coq kernel—a small, trusted, AI-free checker—accepted. Listing an AI as a proof contributor would misrepresent this trust chain. The policy of this dissertation is therefore absolute: AI is a code-scaffolding tool, not an intellectual contributor to formal results.

**Sanctioned seeds**, recorded here for archival completeness: $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, $F_{20}=6765$, $F_{21}=10946$, $L_7=29$, $L_8=47$.

## 4. Results / Evidence

This appendix contains no quantitative experimental results. The following figures are cited here by reference for completeness: 297 Qed theorems (App.A, Ch.18), 63 toks/sec at 92 MHz on XC7A100T (Ch.28), BPB ≤ 1.83 at Gate-2 (Ch.15), 13 Zenodo DOI bundles (App.H). These figures are not re-derived here; they are the results of the scientific chapters they reference.

## 5. Qed Assertions

No Coq theorems are anchored to this appendix; obligations are tracked in the Golden Ledger.

## 6. Sealed Seeds

Inherits the canonical seed pool $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, $F_{20}=6765$, $F_{21}=10946$, $L_7=29$, $L_8=47$.

## 7. Discussion

The AI-assisted disclaimer in Section 3 reflects an evolving norm in formal methods research. As language models become more capable of generating syntactically valid Coq code, the boundary between scaffolding and proof authorship will require ongoing clarification. The Trinity S³AI project's policy—AI for code structure, humans for proof strategy—is one principled position; other positions are possible, provided they are disclosed with equal precision.

The primary limitation of this appendix is that the policy was applied retrospectively to earlier chapters: some scaffolding scripts were generated before the policy was formalised, and it is not possible to recover which specific lines of Coq boilerplate were AI-generated versus hand-written. The 297 Qed proof terms are fully human-verified regardless, but the audit trail for scaffolding code is incomplete. Future work should integrate code-provenance tracking (e.g., cryptographic hashes of AI-generated fragments) from the outset of a project. This appendix connects to App.A (executive summary), Ch.18 (Limitations—Admitted stubs), and App.H (Zenodo DOI registry).

## References

[1] `gHashTag/t27/proofs/canonical/` — Coq canonical proof archive; 65 `.v` files, 297 Qed.

[2] Boldo, S. and Melquiond, G. (2011). Flocq: A Unified Library for Proving Floating-Point Algorithms in Coq. *ARITH 2011*. https://doi.org/10.1109/ARITH.2011.40

[3] Jung, R. et al. (2018). Iris from the Ground Up. *Journal of Functional Programming*, 28, e20. https://doi.org/10.1017/S0956796818000151

[4] This dissertation, Ch.28: FPGA Synthesis — QMTech XC7A100T, 0 DSP, 63 toks/sec, 92 MHz, 1 W.

[5] `gHashTag/trios#411` — App.C Acknowledgments scope issue.

[6] Zenodo DOI bundle B006, 10.5281/zenodo.19227875 — GF16 Probabilistic Format archive.

[7] This dissertation, App.A: Cover + Abstract (250w executive).

[8] This dissertation, App.H: Zenodo DOI Registry (B001–B013).

[9] This dissertation, Ch.18: Limitations — 41 Admitted stubs and Coq.Interval upgrade lane.

[10] This dissertation, Ch.24: Period-Locked Runtime Monitor — IGLA RACE multi-agent system.

[11] Gonthier, G. et al. (2013). A Machine-Checked Proof of the Odd Order Theorem. *ITP 2013*. https://doi.org/10.1007/978-3-642-39634-2_14

[12] Vogel, H. (1979). A better way to construct the sunflower head. *Mathematical Biosciences*, 44(3–4), 179–189. https://doi.org/10.1016/0025-5564(79)90080-4

[13] Leroy, X. (2009). Formal Verification of a Realistic Compiler. *Communications of the ACM*, 52(7), 107–115. https://doi.org/10.1145/1538788.1538814
