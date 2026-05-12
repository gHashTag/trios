![Zenodo DOI registry](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/app-h-zenodo-doi-registry.png)

*Figure — App.H: Zenodo DOI registry (scientific triptych, 1200×800).*

# App.H — Zenodo DOI registry (R5-honest, PASS-6 community-SOT aligned)

> **R5-honest audit, PASS-6 (2026-05-12, community SOT alignment).** This
> appendix was previously titled “13 Zenodo DOI registry” and listed
> bundles B009–B013 against DOIs `10.5281/zenodo.19227881`, `…19227883`,
> `…19227885`, `…19227887`, `…19227889`. Cross-check against the
> declared single source of truth — the Zenodo community
> [`zenodo.org/communities/trinity-s3ai/`](https://zenodo.org/communities/trinity-s3ai/) —
> showed those five DOIs resolve to unrelated third-party works
> (Brazilian pedagogy, Spanish clinical simulation, dermatological
> laser studies, role-semantic invariants in software, energy/employment
> datasets). They are NOT Trinity S³AI artefacts and have been removed.
> The honest registry contains exactly **8 bundles (B001–B008)** that
> are members of the trinity-s3ai community.

## Abstract

Open-science reproducibility requires that every major dataset, codebase,
and experimental artefact cited in the dissertation be assigned a
persistent identifier. This appendix is the authoritative registry of
the Trinity S³AI / GOLDEN SUNFLOWERS Zenodo deposits that are members of
the canonical community
[`zenodo.org/communities/trinity-s3ai/`](https://zenodo.org/communities/trinity-s3ai/)
(12 total community records: 8 description stubs B001–B008 plus 4
deeper-history dataset records D004–D007 referenced in §H.6). The registry
is anchored by the algebraic identity $\varphi^2 + \varphi^{-2} = 3$
(derivable from $\varphi = (1+\sqrt{5})/2$). DOIs B001–B008 span the
range `10.5281/zenodo.19227865` – `10.5281/zenodo.19227879` (odd values
only, one per bundle).

## 1. Introduction

The Trinity S³AI programme generates several distinct classes of
research artefact: trained model weights, FPGA bitstreams, Coq proof
scripts, benchmark corpora, and hardware measurement logs. Each class
must be independently archivable and citable in order to meet
reproducibility standards expected at the dissertation level [1]. The
Zenodo platform, operated by CERN under a CC-BY licence, provides DOI
registration with guaranteed 20-year availability, making it the
appropriate archive for this project [2].

The 8 bundle DOIs registered here correspond to the 8 description
records (B001–B008, sometimes called “seed” deposits) that the author
attached to the trinity-s3ai Zenodo community on 2026-03-26. Larger
binary artefacts (model weights, bitstreams, full Coq archive) are NOT
re-deposited into Zenodo at this time; they remain in the GitHub
mirrors listed in §H.7 and are pointed to from the description records.
The bundle labelling convention is B001–B008, where B stands for
“bundle” and the numeric suffix is sequential by description-stub
order.

## 2. Registry Schema and Metadata Convention

**Definition 2.1 (Bundle record).** Each of the 8 bundle records contains:
- `bundle_id`: B001–B008.
- `doi`: the permanent Zenodo DOI URI.
- `title`: human-readable artefact description.
- `phi_weight`: $\{1.0, 1/\varphi\}$.
- `chapter_links`: the dissertation chapters that cite this bundle.
- `status`: `golden` (member of community `trinity-s3ai`).
- `zenodo_keyword`: `golden-sunflowers; phi^2+phi^-2=3`.

**Convention 2.2 (DOI parity).** All 8 bundle DOIs use odd Zenodo
record numbers (19227865, 19227867, 19227869, 19227871, 19227873,
19227875, 19227877, 19227879). The even-numbered records in the same
range (19227864, …66, …68, …) are **server-side conceptdoi-redirects**
to the same canonical odd records and MUST NOT be cited directly.

**Convention 2.3 (community membership = single source of truth).** A
DOI is part of the Trinity S³AI artefact set if and only if it appears
in the JSON returned by
`GET https://zenodo.org/api/communities/trinity-s3ai/records`. Any
citation outside that list requires an explicit `[R5-honest: external
provenance]` annotation in the citing chapter.

## 3. Full Bundle Descriptions

**B001 — Ternary Neural Networks: Complete Scientific Framework**
(`10.5281/zenodo.19227865`, $\phi$-weight = 1.0).
Description record (v5.0, 2026-03-26) of the HSLM ternary neural network:
27 transformer layers, ternary weights $\{-1,0,+1\}$, $\varphi$-structured
positional embeddings, BPB=1.47 at sequence length $F_{19}=4181$. The
trained `.safetensors` weights and tokeniser live in the GitHub mirror
under `gHashTag/trinity` (§H.7). Chapter links: Ch.28, App.H [4].

**B002 — Zero-DSP FPGA Architecture for Ternary Inference**
(`10.5281/zenodo.19227867`, $\phi$-weight = 1.0).
Description record (v5.0, 2026-03-26) of the QMTech XC7A100T FPGA
implementation. Key metrics: 0 DSP slices, 92 MHz, 63 tokens/sec, 1 W.
The bitstream `.bit` and Vivado project live in the GitHub mirror under
`gHashTag/trinity-fpga` (§H.7). Chapter links: Ch.28, App.F, App.H [5].

**B003 — TRI-27 ISA: Ternary Instruction Set with Coptic Encoding**
(`10.5281/zenodo.19227869`, $\phi$-weight = $1/\varphi \approx 0.618$).
Description record (v5.0, 2026-03-26) of the TRI-27 verifiable VM and
its 27-symbol Coptic ternary alphabet. The Rust VM source and 15
verification test cases live in `gHashTag/t27`. Chapter links: Ch.27,
App.H [6].

**B004 — Queen Lotus Cycle: Autonomous Orchestration for Self-Evolving AI**
(`10.5281/zenodo.19227871`, $\phi$-weight = $1/\varphi$).
Description record (v5.0, 2026-03-26) of the Queen Lotus adaptive
reasoning cycle and its RLHF reward model. Evaluation harness lives in
`gHashTag/trios`. Chapter links: Ch.31, App.H [7].

**B005 — Tri Language: Linear Types, Effects, Dual-Target Compilation**
(`10.5281/zenodo.19227873`, $\phi$-weight = $1/\varphi$).
Description record (v5.0, 2026-03-26) of the Tri language: parser,
typechecker, interpreter, 42 example programs, BNF grammar. Source
lives in `gHashTag/t27/lang/`. Chapter links: Ch.10, App.H [8].

**B006 — Sacred GF16/TF3: Phi-Based Arithmetic for Ternary Computing**
(`10.5281/zenodo.19227875`, $\phi$-weight = 1.0).
Description record (v5.0, 2026-03-26) of the canonical Coq proof
archive — 10 `.v` files containing 48 statements (6 Theorem + 42 Lemma),
35 Qed-proven, 0 Admitted as of the 2026-05-12 audit. The proof files
themselves live in `gHashTag/t27/coq/`. Chapter links: App.B, App.H [9].

**B007 — VSA Operations for Ternary Computing**
(`10.5281/zenodo.19227877`, $\phi$-weight = 1.0).
Description record (v5.0, 2026-03-26) of the Vector-Symbolic-Architecture
binding/unbinding/cleanup operations over GF(16). The HSLM held-out
evaluation corpus (1003 token sequences) lives in `gHashTag/trinity` and
is used in all BPB measurements throughout the dissertation. Chapter
links: Ch.11, Ch.17, Ch.28, App.H [10].

**B008 — Trinity S³AI Framework: Complete Research Collection**
(`10.5281/zenodo.19227879`, $\phi$-weight = $1/\varphi$).
Description record (v5.0, 2026-03-26) — the parent collection record
that links B001–B007 together as one citable framework. Used as the
single-citation handle for survey papers that wish to refer to Trinity
as a whole. Chapter links: every front-matter file; App.H [11].

## 4. Results / Evidence

| Bundle | DOI | $\phi$-weight | Status |
|--------|-----|--------------|--------|
| B001 Ternary NN Framework | 10.5281/zenodo.19227865 | 1.0 | golden |
| B002 FPGA Zero-DSP | 10.5281/zenodo.19227867 | 1.0 | golden |
| B003 TRI-27 ISA | 10.5281/zenodo.19227869 | 0.618 | golden |
| B004 Queen Lotus Cycle | 10.5281/zenodo.19227871 | 0.618 | golden |
| B005 Tri Language | 10.5281/zenodo.19227873 | 0.618 | golden |
| B006 Sacred GF16/TF3 (Coq) | 10.5281/zenodo.19227875 | 1.0 | golden |
| B007 VSA Operations | 10.5281/zenodo.19227877 | 1.0 | golden |
| B008 Framework Parent | 10.5281/zenodo.19227879 | 0.618 | golden |

All 8 DOIs resolve to Zenodo records inside community
`trinity-s3ai` with CC-BY 4.0 licence (community membership re-verified
2026-05-12 via the Zenodo REST API).

## 5. Qed Assertions

No Coq theorems are anchored to this appendix; obligations are tracked
in App.B (Golden Ledger). The B006 Coq archive contains 48 statements,
35 Qed-proven, 0 Admitted (audit 2026-05-12).

## 6. Sealed Seeds

- **B001** (doi, golden, $\phi$-weight = 1.0): `https://doi.org/10.5281/zenodo.19227865` — Ternary NN Framework — linked to Ch.28, App.H.
- **B002** (doi, golden, $\phi$-weight = 1.0): `https://doi.org/10.5281/zenodo.19227867` — FPGA Zero-DSP — linked to Ch.28, App.F, App.H.
- **B003** (doi, golden, $\phi$-weight = 0.618): `https://doi.org/10.5281/zenodo.19227869` — TRI-27 ISA — linked to Ch.27, App.H.
- **B004** (doi, golden, $\phi$-weight = 0.618): `https://doi.org/10.5281/zenodo.19227871` — Queen Lotus Cycle — linked to Ch.31, App.H.
- **B005** (doi, golden, $\phi$-weight = 0.618): `https://doi.org/10.5281/zenodo.19227873` — Tri Language — linked to Ch.10, App.H.
- **B006** (doi, golden, $\phi$-weight = 1.0): `https://doi.org/10.5281/zenodo.19227875` — Sacred GF16/TF3 / Coq — linked to App.B, App.H.
- **B007** (doi, golden, $\phi$-weight = 1.0): `https://doi.org/10.5281/zenodo.19227877` — VSA Operations — linked to Ch.11, Ch.17, Ch.28, App.H.
- **B008** (doi, golden, $\phi$-weight = 0.618): `https://doi.org/10.5281/zenodo.19227879` — Framework Parent — linked to front-matter, App.H.

## 7. Discussion

The 8-bundle registry achieves the dissertation's open-science goal for
the description tier: every major artefact class is independently
citable via a permanent Zenodo DOI inside the canonical community.
Larger binary artefacts (full bitstreams, full Coq archive, raw
benchmark logs, energy traces) remain in GitHub mirrors and inherit
their provenance from the B-series description records.

**Honest scope statement.** Earlier drafts of this appendix listed
13 bundles (B001–B013) and linked B009–B013 to DOIs
`10.5281/zenodo.19227881/3/5/7/9`. R5-honest audit during PASS-6
(2026-05-12) showed those five DOIs resolve to unrelated third-party
works that happened to fall inside the same numeric range:
Brazilian pedagogy practices (19227881/2), Spanish clinical-simulation
study (19227883/4), dermatological laser studies (19227885/6),
role-semantic software invariants (19227887/8), and a structure-first
mathematics monograph (19227889). They are NOT Trinity S³AI deposits
and have been removed from the registry. Any chapter that previously
cited B009–B013 must now either (a) cite the corresponding GitHub mirror
under §H.7, or (b) mint a new Zenodo deposit inside the trinity-s3ai
community and update this appendix in the same PR.

## References

[1] Nosek, B. A. et al. (2015). Promoting an open research culture. *Science*, 348(6242), 1422–1425.

[2] Zenodo. CERN open-data repository. https://zenodo.org. Trinity S³AI single source of truth: https://zenodo.org/communities/trinity-s3ai/

[3] GOLDEN SUNFLOWERS Dissertation, App.B — *Golden Ledger (48 Coq statements, 35 Qed-proven canonical + SHA-1)*.

[4] Zenodo B001: Trinity B001 — Ternary Neural Networks v5.0. DOI: 10.5281/zenodo.19227865.

[5] Zenodo B002: Trinity B002 — Zero-DSP FPGA Architecture v5.0. DOI: 10.5281/zenodo.19227867.

[6] Zenodo B003: Trinity B003 — TRI-27 ISA v5.0. DOI: 10.5281/zenodo.19227869.

[7] Zenodo B004: Trinity B004 — Queen Lotus Cycle v5.0. DOI: 10.5281/zenodo.19227871.

[8] Zenodo B005: Trinity B005 — Tri Language v5.0. DOI: 10.5281/zenodo.19227873.

[9] Zenodo B006: Trinity B006 — Sacred GF16/TF3 v5.0. DOI: 10.5281/zenodo.19227875.

[10] Zenodo B007: Trinity B007 — VSA Operations v5.0. DOI: 10.5281/zenodo.19227877.

[11] Zenodo B008 (parent): Trinity S³AI Framework — Complete Research Collection v5.0. DOI: 10.5281/zenodo.19227879.

[12] gHashTag/trios#430 — App.H ONE SHOT directive (R5-honest, 2026-05-12 community-SOT alignment). GitHub issue.
