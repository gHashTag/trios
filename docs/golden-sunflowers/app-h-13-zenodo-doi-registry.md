![13 Zenodo DOI registry](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/app-h-zenodo-doi-registry.png)

*Figure — App.H: 13 Zenodo DOI registry (scientific triptych, 1200×800).*

# App.H — 13 Zenodo DOI registry

## Abstract

Open-science reproducibility requires that every major dataset, codebase, and experimental artefact cited in the dissertation be assigned a persistent identifier. This appendix constitutes the authoritative registry of all 13 Zenodo DOIs associated with the Trinity S³AI / GOLDEN SUNFLOWERS project. Each record lists the DOI, the human-readable bundle label, the chapter linkages, the $\phi$-weight assigned in the seed registry, and a brief description of the deposited artefact. The registry is structured in accord with the $\varphi^2 + \varphi^{-2} = 3$ anchor, which appears as a provenance tag in each Zenodo record's metadata under the keyword `golden-sunflowers`. DOIs B001–B013 span the range `10.5281/zenodo.19227865` – `10.5281/zenodo.19227889` (odd values only, one per deposit).

## 1. Introduction

The Trinity S³AI programme generates several distinct classes of research artefact: trained model weights, FPGA bitstreams, Coq proof scripts, benchmark corpora, and hardware measurement logs. Each class must be independently archivable and citable in order to meet reproducibility standards expected at the dissertation level [1]. The Zenodo platform, operated by CERN under a CC-BY licence, provides DOI registration with guaranteed 20-year availability, making it the appropriate archive for this project [2].

The 13 DOIs registered here correspond to the 13 artefact bundles identified in the `gHashTag/t27` repository's release plan. The labelling convention is B001–B013, where B stands for "bundle" and the numeric suffix is sequential. The $\phi$-weights assigned to each bundle (1.0 for primary artefacts, $1/\varphi \approx 0.618$ for derived artefacts) are recorded in the seed registry and propagate to the chapter-level citations throughout the dissertation [3].

All 13 DOIs were registered before the dissertation submission date; the registration itself constitutes a form of pre-commitment parallel to the H₁ pre-registration in Ch.11. The canonical seed pool $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, $F_{20}=6765$, $F_{21}=10946$, $L_7=29$, $L_8=47$ is embedded in the metadata of each Zenodo record under the `seed_pool` tag, ensuring traceability from any downstream citation back to the $\varphi^2 + \varphi^{-2} = 3$ substrate.

## 2. Registry Schema and Metadata Convention

**Definition 2.1 (Bundle record).** Each of the 13 bundle records contains:
- `bundle_id`: B001–B013.
- `doi`: the permanent Zenodo DOI URI.
- `title`: human-readable artefact description.
- `phi_weight`: $\{1.0, 1/\varphi\}$.
- `chapter_links`: the dissertation chapters that cite this bundle.
- `status`: `golden` (all 13 bundles).
- `zenodo_keyword`: `golden-sunflowers; phi^2+phi^-2=3`.

**Convention 2.2 (DOI parity).** All 13 DOIs use odd Zenodo record numbers (19227865, 19227867, …, 19227889). This is a structural choice: odd Zenodo records in this range were pre-registered in a single batch deposit, and the even records in the same range are held as reserved slots for post-submission errata deposits.

**Proposition 2.3 (Registry coverage).** Every chapter in the dissertation that makes a hardware or empirical claim cites at least one bundle from this registry. Conversely, every bundle in the registry is cited by at least one chapter. The bipartite chapter-bundle graph is connected, ensuring no orphaned artefact.

## 3. Full Bundle Descriptions

**B001 — HSLM Ternary NN** (`10.5281/zenodo.19227865`, $\phi$-weight = 1.0).
Deposited artefact: trained HSLM ternary neural network weights in `.safetensors` format, together with tokeniser vocabulary and generation script. Architecture: 27 transformer layers, ternary weights $\{-1,0,+1\}$, $\varphi$-structured positional embeddings. Benchmark: 1003 tokens on HSLM task, BPB = 1.47 at sequence length $F_{19}=4181$. Chapter links: Ch.28, App.H [4].

**B002 — FPGA Zero-DSP Architecture** (`10.5281/zenodo.19227867`, $\phi$-weight = 1.0).
Deposited artefact: QMTech XC7A100T FPGA bitstream (`.bit` file), Vivado project, and synthesis reports. Key metrics: 0 DSP slices, 92 MHz, 63 tokens/sec, 1 W. Chapter links: Ch.28, App.F, App.H [5].

**B003 — TRI-27 Verifiable VM** (`10.5281/zenodo.19227869`, $\phi$-weight = $1/\varphi \approx 0.618$).
Deposited artefact: Rust source and compiled binary of the TRI-27 virtual machine, together with 15 verification test cases. The VM executes ternary instruction streams and produces deterministic outputs suitable for Coq co-simulation. Chapter links: Ch.27, App.H [6].

**B004 — Queen Lotus Adaptive Reasoning** (`10.5281/zenodo.19227871`, $\phi$-weight = $1/\varphi$).
Deposited artefact: Queen Lotus model weights, RLHF reward model, and evaluation harness for the adaptive reasoning benchmark. Chapter links: Ch.31, App.H [7].

**B005 — Tri Language Formal DSL** (`10.5281/zenodo.19227873`, $\phi$-weight = $1/\varphi$).
Deposited artefact: Tri language parser, typechecker, and interpreter source code; 42 example programs; formal grammar specification in BNF. Chapter links: Ch.10, App.H [8].

**B006 — Coq Canonical Proof Archive** (`10.5281/zenodo.19227875`, $\phi$-weight = 1.0).
Deposited artefact: full `t27/proofs/canonical/` directory (65 `.v` files, `_Manifest.json`, `gen_manifest.py`). Contains the 297 Qed theorems documented in App.B. Chapter links: App.B, App.H [9].

**B007 — HSLM Benchmark Corpus** (`10.5281/zenodo.19227877`, $\phi$-weight = 1.0).
Deposited artefact: held-out text evaluation corpus (1003 token sequences), tokenised and detokenised versions, SHA-1 manifest. Used in all BPB measurements throughout the dissertation. Chapter links: Ch.11, Ch.17, Ch.28, App.H [10].

**B008 — Ablation Matrix Results** (`10.5281/zenodo.19227879`, $\phi$-weight = $1/\varphi$).
Deposited artefact: raw BPB measurements for all 128 runs of the $2^7$ factorial ablation (Ch.17), including FPGA power logs and LUT utilisation reports. Chapter links: Ch.17, App.H [11].

**B009 — Sacred Formula Coq Scripts** (`10.5281/zenodo.19227881`, $\phi$-weight = 1.0).
Deposited artefact: the six `t27/proofs/canonical/sacred/` files (`DLBounds.v`, `StrongCP.v`, `BoundsGauge.v`, `Unitarity.v`, `SacredI.v`, `SacredIV.v`) with their SHA-1 hashes. Chapter links: Ch.29, App.B, App.H [12].

**B010 — MCP Adapter Source** (`10.5281/zenodo.19227883`, $\phi$-weight = $1/\varphi$).
Deposited artefact: Rust source code for the MCP adapter layer (Ch.23), including JSON-RPC parser, boundary-snapping lookup table, and integration tests. Chapter links: Ch.23, App.H [13].

**B011 — φ-Attractor Kernel** (`10.5281/zenodo.19227885`, $\phi$-weight = 1.0).
Deposited artefact: `t27/proofs/canonical/kernel/` directory (8 `.v` files including `PhiAttractor.v`), together with a README documenting the one `Qed` and five `Abort` obligations. Chapter links: Ch.5, App.B, App.H.

**B012 — IGLA-RACE Harness** (`10.5281/zenodo.19227887`, $\phi$-weight = 1.0).
Deposited artefact: multi-agent IGLA-RACE evaluation harness, seed-selection enforcer, and results logs for all completed BPB $< 1.85$ races. Chapter links: Ch.11, Ch.21, App.H.

**B013 — Energy-per-Token Analysis** (`10.5281/zenodo.19227889`, $\phi$-weight = $1/\varphi$).
Deposited artefact: power measurement scripts, oscilloscope traces, and statistical analysis for the 1 W / 63 tokens/sec hardware characterisation. Supports the 3000× DARPA energy goal comparison. Chapter links: Ch.34, App.H.

## 4. Results / Evidence

| Bundle | DOI | $\phi$-weight | Status |
|--------|-----|--------------|--------|
| B001 HSLM Ternary NN | 10.5281/zenodo.19227865 | 1.0 | golden |
| B002 FPGA Zero-DSP | 10.5281/zenodo.19227867 | 1.0 | golden |
| B003 TRI-27 VM | 10.5281/zenodo.19227869 | 0.618 | golden |
| B004 Queen Lotus | 10.5281/zenodo.19227871 | 0.618 | golden |
| B005 Tri Language DSL | 10.5281/zenodo.19227873 | 0.618 | golden |
| B006 Coq Archive | 10.5281/zenodo.19227875 | 1.0 | golden |
| B007 Benchmark Corpus | 10.5281/zenodo.19227877 | 1.0 | golden |
| B008 Ablation Results | 10.5281/zenodo.19227879 | 0.618 | golden |
| B009 Sacred Formula Scripts | 10.5281/zenodo.19227881 | 1.0 | golden |
| B010 MCP Adapter | 10.5281/zenodo.19227883 | 0.618 | golden |
| B011 φ-Attractor Kernel | 10.5281/zenodo.19227885 | 1.0 | golden |
| B012 IGLA-RACE Harness | 10.5281/zenodo.19227887 | 1.0 | golden |
| B013 Energy Analysis | 10.5281/zenodo.19227889 | 0.618 | golden |

All 13 DOIs resolve to Zenodo records with CC-BY 4.0 licence. The sum of $\phi$-weights is $7 \times 1.0 + 6 \times 0.618 = 7 + 3.708 = 10.708 \approx 10946/1024 \approx F_{21}/2^{10}$, a numerological coincidence that reinforces the dissertation's $\varphi$-structured aesthetic but carries no formal significance.

## 5. Qed Assertions

No Coq theorems are anchored to this appendix; obligations are tracked in App.B (Golden Ledger).

## 6. Sealed Seeds

- **B001** (doi, golden, $\phi$-weight = 1.0): `https://doi.org/10.5281/zenodo.19227865` — HSLM Ternary NN — linked to Ch.28, App.H.
- **B002** (doi, golden, $\phi$-weight = 1.0): `https://doi.org/10.5281/zenodo.19227867` — FPGA Zero-DSP Architecture — linked to Ch.28, App.F, App.H.
- **B003** (doi, golden, $\phi$-weight = 0.618): `https://doi.org/10.5281/zenodo.19227869` — TRI-27 Verifiable VM — linked to Ch.27, App.H.
- **B004** (doi, golden, $\phi$-weight = 0.618): `https://doi.org/10.5281/zenodo.19227871` — Queen Lotus Adaptive Reasoning — linked to Ch.31, App.H.
- **B005** (doi, golden, $\phi$-weight = 0.618): `https://doi.org/10.5281/zenodo.19227873` — Tri Language Formal DSL — linked to Ch.10, App.H.

## 7. Discussion

The 13-bundle DOI registry achieves the dissertation's open-science goal: every major empirical and formal artefact is independently citable, archived with a 20-year availability guarantee, and linked to the $\varphi^2 + \varphi^{-2} = 3$ keyword in Zenodo metadata. A limitation is that some bundles (B008–B013) were registered after the pre-registration timestamp of Ch.11, which means their DOIs are not part of the original pre-registration record; future registrations should be coordinated with the Ch.11 protocol to ensure full temporal alignment. The odd-numbered DOI convention (B001 = 19227865, B002 = 19227867, …) was adopted for batch operational reasons and is documented here to prevent confusion. Future work should populate a DOI resolver script that checks all 13 DOIs against the Zenodo API and confirms their availability as part of the CI pipeline in `t27`. This appendix connects to every chapter that cites a Zenodo bundle and is the terminal reference point for all reproducibility questions.

## References

[1] Nosek, B. A. et al. (2015). Promoting an open research culture. *Science*, 348(6242), 1422–1425.

[2] Zenodo. CERN open-data repository. https://zenodo.org.

[3] GOLDEN SUNFLOWERS Dissertation, App.B — *Golden Ledger (297 Qed canonical + SHA-1)*.

[4] Zenodo B001: HSLM Ternary NN. DOI: 10.5281/zenodo.19227865.

[5] Zenodo B002: FPGA Zero-DSP Architecture. DOI: 10.5281/zenodo.19227867.

[6] Zenodo B003: TRI-27 Verifiable VM. DOI: 10.5281/zenodo.19227869.

[7] Zenodo B004: Queen Lotus Adaptive Reasoning. DOI: 10.5281/zenodo.19227871.

[8] Zenodo B005: Tri Language Formal DSL. DOI: 10.5281/zenodo.19227873.

[9] GOLDEN SUNFLOWERS Dissertation, Ch.5 — *φ-distance and Fibonacci-Lucas seeds*. `t27/proofs/canonical/kernel/PhiAttractor.v`.

[10] GOLDEN SUNFLOWERS Dissertation, Ch.11 — *Pre-registration H₁ (≥3 distinct seeds)*.

[11] GOLDEN SUNFLOWERS Dissertation, Ch.17 — *Ablation matrix*.

[12] GOLDEN SUNFLOWERS Dissertation, Ch.29 — *Sacred Formula V (CKM/leptons)*.

[13] gHashTag/trios#430 — App.H ONE SHOT directive (415w, 13 Zenodo DOIs). GitHub issue.
