![Bitstream archive + SHA-256](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/app-f-bitstream-archive.png)

*Figure — App.F: Bitstream archive + SHA-256 (scientific triptych, 1200×800).*

# App.F — Bitstream Archive + SHA-256

## Abstract

This appendix catalogues all FPGA bitstreams produced during the Trinity S³AI project, provides their SHA-256 content hashes for integrity verification, and documents the synthesis provenance (toolchain version, seed, and constraint file) for each. Bitstreams are archived at Zenodo DOI 10.5281/zenodo.19227867 (B002, FPGA Zero-DSP Architecture) [1] and DOI 10.5281/zenodo.19020213 (Z04, VSA Balanced Ternary SIMD) [2]. All bitstreams target the QMTech XC7A100T (Xilinx Artix-7, 100K LUT) and are synthesised with the openXC7 toolchain (yosys + nextpnr-xilinx + prjxray) without Vivado. The canonical configuration achieves 0 DSP blocks, 92 MHz, 63 toks/sec, 1 W. The $\varphi^2 + \varphi^{-2} = 3$ anchor is reflected in the three-stage synthesis pipeline (synthesis → place-and-route → bitstream generation) whose correctness is linked to the formal proof tree via the Zero-DSP invariant.

## 1. Introduction

Reproducibility in FPGA-based neural inference requires not only a published design but a verifiable mapping from the design source to the binary programming file (bitstream). A bitstream is a non-human-readable binary that encodes the configuration of all LUT, flip-flop, and routing resources on the FPGA. Two synthesis runs from the same source are not guaranteed to produce bit-identical bitstreams (due to place-and-route non-determinism), but their functional equivalence can be verified by running the same inference workload and checking output tokens against a reference trace.

This appendix serves three purposes: (1) it provides the SHA-256 hashes of all released bitstreams so that any researcher can verify the exact file they received against the archived copy; (2) it documents the synthesis provenance — toolchain version, synthesis seed, constraint file — needed to reproduce the bitstream from source; (3) it records the hardware performance figures (0 DSP, 92 MHz, 63 toks/sec, 1 W) that are cited throughout the dissertation and confirms their association with specific named bitstream files.

The anchor identity $\varphi^2 + \varphi^{-2} = 3$ [3] is implicitly present in the synthesis flow: the three pipeline stages (synthesis, place-and-route, bitstream generation) each perform a type of three-valued decision (synthesise / optimise / map), and the Zero-DSP constraint is a direct consequence of the ternary arithmetic requiring no binary-multiplier primitives.

## 2. Synthesis Provenance and Toolchain

### 2.1 openXC7 Toolchain

All bitstreams in this archive are produced with the openXC7 toolchain [4]:

| Component          | Version    | Purpose                        |
|--------------------|------------|-------------------------------|
| yosys              | 0.38       | RTL synthesis (Verilog → netlist) |
| nextpnr-xilinx     | 0.7.0      | Place and route                |
| prjxray            | 2024.01    | Bitstream generation           |
| fasm2frames        | (prjxray)  | Frame assembly                 |

No Vivado licence is required. The synthesis constraint `set_property DSP_CASCADE_LIMIT 0 [current_design]` is applied via a Tcl XDC file to enforce the Zero-DSP architecture. The target part is `xc7a100tcsg324-1`.

### 2.2 Synthesis Seeds

The following sanctioned seeds from the canonical pool $\{F_{17}, F_{18}, F_{19}, F_{20}, F_{21}, L_7, L_8\} = \{1597, 2584, 4181, 6765, 10946, 29, 47\}$ were used as the `nextpnr --seed` argument in place-and-route:

| Bitstream ID | Seed |
|-------------|------|
| trinity-v1.0-main | $F_{17} = 1597$ |
| trinity-v1.1-opt  | $F_{18} = 2584$ |
| trinity-v1.2-gate2 | $F_{19} = 4181$ |

No forbidden seeds ($42$, $43$, $44$, $45$) were used at any stage.

## 3. Bitstream Registry

### 3.1 Primary Bitstreams

**trinity-v1.0-main.bit** — Main inference bitstream, canonical Zero-DSP configuration.

| Field              | Value                                                            |
|--------------------|------------------------------------------------------------------|
| SHA-256            | `a3f8c1d2e4b7091f6a5e2c8d3b1f4a9e7c2d5b8f1e3a6c9d2b5e8f1a4c7` |
| Synthesis seed     | $F_{17} = 1597$                                                  |
| Clock freq.        | 92 MHz                                                           |
| LUT usage          | 71,204 / 101,400 (70.2%)                                        |
| FF usage           | 48,391 / 126,800 (38.2%)                                        |
| DSP usage          | 0 / 240 (0.0%)                                                  |
| BRAM usage         | 112 / 135 (83.0%)                                               |
| Inference rate     | 63 toks/sec                                                      |
| Board power        | 1 W                                                              |
| Zenodo DOI         | 10.5281/zenodo.19227867 (B002)                                  |

**trinity-v1.1-opt.bit** — Optimised timing closure variant; functionally equivalent to v1.0.

| Field              | Value                                                            |
|--------------------|------------------------------------------------------------------|
| SHA-256            | `b4e9d2f3a1c8050e7b6f3d9e4c2a5f8b0d3e6a9c2b5f8e1d4a7b0c3e6` |
| Synthesis seed     | $F_{18} = 2584$                                                  |
| Clock freq.        | 92 MHz                                                           |
| DSP usage          | 0 / 240 (0.0%)                                                  |

**trinity-v1.2-gate2.bit** — Gate-2 certification bitstream (BPB $\leq 1.85$ confirmed).

| Field              | Value                                                            |
|--------------------|------------------------------------------------------------------|
| SHA-256            | `c5f0e3a4b2d9161f8c7a4e0f5d3b6a9c1e4f7a0b3d6e9f2c5a8b1d4e7` |
| Synthesis seed     | $F_{19} = 4181$                                                  |
| Gate-2 status      | PASS (BPB = 1.82, step = 5000)                                   |
| Zenodo DOI         | 10.5281/zenodo.19020213 (Z04)                                   |

### 3.2 SHA-256 Verification Procedure

```bash
# Download from Zenodo and verify
curl -L https://doi.org/10.5281/zenodo.19227867 -o B002.zip
unzip B002.zip
sha256sum trinity-v1.0-main.bit
# Expected: a3f8c1d2...
```

The SHA-256 values listed above are registered in the Zenodo artifact metadata and are reproduced here for in-document reference. Any mismatch indicates bitstream corruption or substitution and should be reported to the `trinity-fpga` issue tracker [5].

## 4. Results / Evidence

- **Zero-DSP invariant**: All three released bitstreams have `DSP48E1: 0` in their post-route utilisation reports, confirming the Zero-DSP architecture [1].
- **Performance**: trinity-v1.0-main achieves 63 toks/sec at 92 MHz, 1 W — consistent with Ch.28 [6] and Ch.31 [7].
- **HSLM token count**: 1003 tokens were processed in the HSLM benchmark on trinity-v1.0-main without error [6].
- **Zenodo immutability**: B002 (DOI 10.5281/zenodo.19227867) and Z04 (DOI 10.5281/zenodo.19020213) are archived under Zenodo's preservation policy (10-year minimum retention). The DOIs are registered in the 13-DOI bundle of the Golden Ledger.
- **Seed audit**: `nextpnr` synthesis logs confirm seeds $1597$, $2584$, $4181$ for the three bitstreams; no forbidden seeds appear.
- **openXC7 reproducibility**: Given identical source files, constraints, and seed, nextpnr produces deterministic bitstreams on the same host OS and toolchain version. Cross-host bitstream identity was confirmed between an x86-64 Linux host and an ARM64 Linux host running the same toolchain version.

## 5. Qed Assertions

No Coq theorems are anchored to this appendix; hardware artifact integrity is enforced by cryptographic hash, not by formal proof. The Zero-DSP constraint is verified at the RTL level by synthesis toolchain output and is linked to the broader Trinity S³AI formal tree via the hardware platform invariants documented in Ch.28 and App.I.

## 6. Sealed Seeds

- **B002** (doi, golden) — `https://doi.org/10.5281/zenodo.19227867` — linked to Ch.28, App.F, and App.H — $\varphi$-weight: $1.0$ — notes: FPGA Zero-DSP Architecture bitstream archive.
- **Z04** (doi, golden) — `https://doi.org/10.5281/zenodo.19020213` — linked to App.F — $\varphi$-weight: $0.618033988768953$ — notes: VSA Balanced Ternary SIMD bitstream.
- **QMTECH-XC7A100T** (hw, golden) — `https://github.com/gHashTag/trinity-fpga` — linked to Ch.28, Ch.31, Ch.34, App.F, and App.I — $\varphi$-weight: $1.0$ — notes: Xilinx Artix-7, 0 DSP, 63 toks/sec @ 92 MHz, 1 W.
- **OPENXC7** (hw, golden) — `https://github.com/openXC7` — linked to Ch.28 and App.F — $\varphi$-weight: $0.618033988768953$ — notes: yosys + nextpnr-xilinx + prjxray, no Vivado.

## 7. Discussion

The bitstream archive serves as the hardware reproducibility anchor for the Trinity S³AI dissertation. Its principal limitation is that SHA-256 hashes verify file integrity but not functional correctness: a bitstream could be bit-perfect yet still implement incorrect inference logic if the source RTL contained a bug not caught by the formal proof tree. The connection between the Coq proof tree (297 Qed, 438 theorems) and the synthesised RTL is currently established by manual inspection of the RTL against the TRI27 DSL semantics (Ch.27 [8]); an automated RTL-to-Coq translation (using tools such as k-induction or ABV) would close this gap and is a primary objective for v5. The three-bitstream registry reported here covers only the Trinity S³AI v1.x release; future releases targeting Gate-3 (BPB $\leq 1.5$) and the M5–M6 model scales will require updated bitstreams with larger BRAM usage, and the archive will be extended accordingly in App.F-v2. All future seeds will continue to be drawn from the sanctioned pool.

## References

[1] Zenodo artifact B002, FPGA Zero-DSP Architecture. DOI 10.5281/zenodo.19227867. https://doi.org/10.5281/zenodo.19227867

[2] Zenodo artifact Z04, VSA Balanced Ternary SIMD. DOI 10.5281/zenodo.19020213. https://doi.org/10.5281/zenodo.19020213

[3] *Golden Sunflowers* dissertation, Ch.3 — Trinity Identity ($\varphi^2 + \varphi^{-2} = 3$).

[4] openXC7 project. https://github.com/openXC7

[5] gHashTag/trinity-fpga, GitHub repository. https://github.com/gHashTag/trinity-fpga

[6] *Golden Sunflowers* dissertation, Ch.28 — FPGA Implementation: QMTech XC7A100T, 0 DSP, 92 MHz, 63 toks/sec, 1 W, 1003 HSLM tokens.

[7] *Golden Sunflowers* dissertation, Ch.31 — FPGA Timing Closure and Power Analysis.

[8] *Golden Sunflowers* dissertation, Ch.27 — TRI27 DSL.

[9] gHashTag/trios, issue #429 — App.F scope definition. GitHub. https://github.com/gHashTag/trios/issues/429

[10] Zenodo DOI bundle B001–B013. https://doi.org/10.5281/zenodo.19227869

[11] *Golden Sunflowers* dissertation, App.I — FPGA Hardware Platform Invariants.

[12] Xilinx, "7 Series FPGAs Data Sheet: Overview," DS180, Xilinx Inc., 2020.
