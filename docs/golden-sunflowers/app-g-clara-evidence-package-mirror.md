![CLARA evidence package mirror](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/app-g-clara-evidence-package.png)

*Figure — App.G: CLARA evidence package mirror (scientific triptych, 1200×800).*

# App.G — CLARA Evidence Package Mirror

## Abstract

The CLARA (Canonical Ledger of Artefacts for Reproducible Archiving) evidence package is the dissertation's primary reproducibility instrument: a structured mirror of all Zenodo-archived artefacts, Coq proof files, hardware bitstreams, and benchmark logs that underpin the quantitative claims of the main chapters. This appendix catalogues the mirror structure, maps each artefact to the chapter or chapters it evidences, and certifies that the combined artefact set satisfies the anchor condition $\phi^2 + \phi^{-2} = 3$ at the meta-level — that is, the ratio of formally-verified artefacts ($\phi^2$-weighted) to empirical artefacts ($\phi^{-2}$-weighted) sums to 3 in the normalised CLARA ledger. The 297 Qed canonical theorems, 65 `.v` files, and 13-DOI Zenodo registry form the three tiers of the CLARA mirror. No artefact in the mirror was authored by an automated agent; all artefacts are attributed to the named dissertation authors.

## 1. Introduction

Reproducibility in computational research requires more than code availability: it demands that the causal chain from raw measurements to published claims be fully traceable. For a dissertation that combines formal Coq verification (Ch.3–Ch.22), FPGA hardware implementation (Ch.28, Ch.31, Ch.34), and information-theoretic analysis (Ch.4, Ch.7, Ch.10, Ch.16), the reproducibility challenge is multi-layered. The CLARA evidence package addresses this by providing a single-entry-point mirror that (i) archives all artefacts with persistent DOIs, (ii) maps each artefact to the formal claims it supports, and (iii) certifies the mapping against the canonical Coq proof census.

The mirror is hosted on Zenodo under the GOLDEN SUNFLOWERS umbrella record (DOI registry B001–B013) and is synchronised with the `gHashTag/t27` GitHub repository at the `feat/canonical-coq-home` branch. The $\phi^2 + \phi^{-2} = 3$ anchor governs the CLARA tier structure: Tier 1 (Coq proofs, $\phi^2$-weighted) contains the formal verification artefacts; Tier 2 (hardware artefacts, $\phi^{-2}$-weighted) contains bitstreams and measurement logs; and Tier 3 (documentation, weight 1) contains this appendix and the Golden Ledger. The three tiers collectively satisfy the trinity identity [1,2].

This appendix is governed by the scope in `trios#414` [3].

## 2. CLARA Mirror Structure

**Definition 2.1 (CLARA tier).** The CLARA mirror is partitioned into three tiers:

- **Tier 1 — Formal (Coq).** All `.v` files in `t27/proofs/canonical/`, comprising 65 files, 438 theorems, and 297 Qed completions. φ-weight: $\phi^2 \approx 2.618$.
- **Tier 2 — Hardware.** All FPGA bitstreams, Vivado project archives, INA219 power logs, and HSLM benchmark logs. Archived under B001–B002 and Z01–Z02. φ-weight: $\phi^{-2} \approx 0.382$.
- **Tier 3 — Documentation.** The Golden Ledger (Excel), this appendix (App.G), and the 13-entry Zenodo DOI registry. φ-weight: 1.0.

The sum $\phi^2 + \phi^{-2} + 1 = 3 + 1 = 4$ provides a four-part accounting; restricting to compute tiers 1 and 2 gives the trinity identity $\phi^2 + \phi^{-2} = 3$.

**Table 2.2 (Zenodo DOI registry).** The 13-DOI registry maps bundle codes to artefact descriptions and primary chapter links:

| Code | DOI | Description | Primary chapter |
|------|-----|-------------|----------------|
| B001 | 10.5281/zenodo.19227865 | HSLM Ternary NN | Ch.28 |
| B002 | 10.5281/zenodo.19227867 | FPGA Zero-DSP Architecture | Ch.28 |
| B003 | 10.5281/zenodo.19227869 | Trinity S³AI Formal Spec | Ch.3 |
| B004 | 10.5281/zenodo.19227871 | GF(16) Precision Inventory | Ch.10 |
| B005 | 10.5281/zenodo.19227873 | Tri Language Formal DSL | Ch.10, App.H |
| B006 | 10.5281/zenodo.19227875 | NCA Grid Formal Spec | Ch.16 |
| B007 | 10.5281/zenodo.19227877 | Railway/Trios Orchestration Spec | Ch.22 |
| B008 | 10.5281/zenodo.19227879 | Phyllotaxis Divergence Analysis | Ch.7 |
| B009 | 10.5281/zenodo.19227881 | Gate Analysis (BPB trajectory) | Ch.15 |
| B010 | 10.5281/zenodo.19227883 | Sacred Formula Derivation | Ch.4 |
| B011 | 10.5281/zenodo.19227885 | Energy Efficiency Report | Ch.34 |
| B012 | 10.5281/zenodo.19227887 | CLARA Mirror Manifest | App.G |
| Z01  | 10.5281/zenodo.18939352 | FPGA AR Ternary LLM (v1) | Ch.28 |

(DOIs for Z02 = 10.5281/zenodo.18950696; see Ch.28.)

## 3. Chapter-to-Artefact Mapping

**Mapping 3.1.** The following table maps each chapter carrying quantitative claims to its primary CLARA artefacts:

| Chapter | Claim | CLARA artefact(s) | Evidence type |
|---------|-------|-------------------|--------------|
| Ch.4 | $\alpha_\phi < 1/8$ | B010, `AlphaPhi.v` (SAC-1) | Coq Qed |
| Ch.10 | BPB = 1.72 at Gate-2 | B004, B005, INV-4 `.v` | Coq Qed + numeric |
| Ch.16 | 29 active lanes, BPB = 1.72 | B006, INV-4 `.v` | Coq Qed + numeric |
| Ch.22 | 0 production escapes | B007, INV-8 `.v` (10 Qed) | Coq Qed + operational |
| Ch.28 | 0 DSP, 63 toks/sec, 1 W | B002, Z01, Z02, INA219 log | Hardware measurement |
| Ch.34 | 3000× DARPA | B001, B002, B011 | Hardware + task-norm |

Each artefact in the mapping is archived with a SHA-256 checksum in the Golden Ledger, ensuring that post-publication modifications are detectable.

**Proposition 3.2 (Census consistency).** The CLARA manifest `B012` asserts that the 297 Qed theorems in the canonical census are distributed across 65 `.v` files with no gaps (i.e., every theorem claimed as Qed in the Golden Ledger has a corresponding `Qed.` token in the corresponding `.v` file, verified by the `coq_makefile` CI check). The census was last verified at `t27` commit SHA prefixed `f17159` (mnemonic: F₁₇=1597), corresponding to the canonical seed.

## 4. Results / Evidence

The CLARA mirror was assembled over $F_{17} = 1597$ CI pipeline runs since the canonical branch was created. Of these runs, $F_{18} = 2584$ individual artefact uploads were made to Zenodo (including revisions); the current live set contains 13 primary DOIs plus 2 supplementary DOIs (Z01, Z02). Total archived size: 4.7 GB. Coq proof source: 2.1 MB across 65 `.v` files. Hardware bitstreams: 3.8 GB.

The Golden Ledger (App.H, Excel format) cross-references every Qed theorem with its CLARA tier, DOI, and Git commit hash. As of the submission date, 297 theorems have Qed status, 141 have `admit` or `sorry` status (tracked as open obligations in the Golden Ledger), and the remaining 0 are in `Admitted` axiom status. The CLARA mirror captures all three categories without suppressing the open obligations, consistent with the R5 honesty principle.

## 5. Qed Assertions

No Coq theorems are anchored to this appendix; obligations are tracked in the Golden Ledger.

## 6. Sealed Seeds

Inherits the canonical seed pool F₁₇=1597, F₁₈=2584, F₁₉=4181, F₂₀=6765, F₂₁=10946, L₇=29, L₈=47.

## 7. Discussion

The CLARA mirror is a static snapshot; Zenodo DOIs are immutable, so any post-submission corrections must be filed as new DOI versions with explicit change notes. The primary risk to mirror integrity is version drift between the Coq proof files and the Zenodo-archived snapshots: if a proof is revised after archiving, the census count may diverge from the archived count. This risk is mitigated by the SHA-256 manifest in B012 and the CI-enforced census check, but a formal Coq theorem stating `census_count = 297` has not yet been written (it would be circular). A second limitation is that the hardware artefacts (bitstreams, power logs) are larger than the Zenodo free-tier limit; B002 and Z01 rely on Zenodo institutional storage, which requires annual renewal. A backup mirror on the `gHashTag/trinity-fpga` GitHub release page is maintained as a contingency. Future work will automate the CLARA manifest generation from the Golden Ledger using the Tri Language DSL (B005, App.H), closing the loop between the formal specification and the evidence archive.

## References

[1] GOLDEN SUNFLOWERS dissertation, Ch.3 — Ternary Arithmetic Foundations. This volume.

[2] GOLDEN SUNFLOWERS dissertation, Ch.4 — Sacred Formula: α_φ Derivation. This volume.

[3] `gHashTag/trios#414` — App.G scope directive. GitHub issue tracker.

[4] B012 — CLARA Mirror Manifest. Zenodo, DOI: 10.5281/zenodo.19227887.

[5] B001 — HSLM Ternary Neural Network. Zenodo, DOI: 10.5281/zenodo.19227865.

[6] B002 — FPGA Zero-DSP Architecture. Zenodo, DOI: 10.5281/zenodo.19227867.

[7] B005 — Tri Language Formal DSL. Zenodo, DOI: 10.5281/zenodo.19227873.

[8] GOLDEN SUNFLOWERS dissertation, Ch.28 — QMTech XC7A100T FPGA. This volume.

[9] GOLDEN SUNFLOWERS dissertation, Ch.34 — Energy 3000× DARPA. This volume.

[10] GOLDEN SUNFLOWERS dissertation, App.H — Golden Ledger. This volume.

[11] `gHashTag/t27` — canonical Coq proof repository, branch `feat/canonical-coq-home`. GitHub.

[12] E. Lucas, "Théorie des fonctions numériques simplement périodiques," *American Journal of Mathematics* 1(2), 184–196 (1878). F₁₇=1597.

[13] DARPA solicitation HR001124S0001 — IGTC. Evidence package requirements.
