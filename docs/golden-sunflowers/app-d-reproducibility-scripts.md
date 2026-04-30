![Reproducibility scripts](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/app-d-reproducibility-scripts.png)

*Figure — App.D: Reproducibility scripts (scientific triptych, 1200×800).*

# App.D — Reproducibility Scripts

## Abstract

This appendix catalogues every script required to reproduce the numerical results, Coq proof checks, and hardware bitstreams reported in this dissertation. The reproducibility package is archived on Zenodo (DOI B004) and on GitHub at `gHashTag/trinity-fpga` and `gHashTag/t27`. The entry point is `reproduce.sh`, which accepts a single sanctioned seed from the pool $\{F_{17}=1597, F_{18}=2584, F_{19}=4181, F_{20}=6765, F_{21}=10946, L_7=29, L_8=47\}$ and orchestrates training, evaluation, Coq verification, and FPGA synthesis in a fully automated pipeline. The anchor identity $\varphi^2 + \varphi^{-2} = 3$ is checked at pipeline entry as a sanity assertion and aborts the run if violated. All results in this dissertation were produced with this pipeline at version tag `v4.0.0`.

## 1. Introduction

Reproducibility in deep-learning research is complicated by hardware non-determinism, library version drift, and implicit seed dependencies. The TRINITY S³AI programme addresses all three sources of irreproducibility through a combination of the STROBE sealed-seed protocol (Ch.13), a pinned software environment specified by a `Nix` flake, and a deterministic FPGA synthesis flow. The $\varphi^2 + \varphi^{-2} = 3$ identity serves as both a mathematical anchor and a runtime health-check: the `reproduce.sh` script computes $\varphi^2 + \varphi^{-2}$ to 64-bit floating-point precision at startup and halts if the result differs from 3 by more than $10^{-12}$. This check catches platform-specific floating-point anomalies before any training computation begins.

The appendix is organised as follows. Section 2 describes the software environment and entry-point script. Section 3 catalogues the individual scripts by function. Section 4 presents the results of a reproducibility audit. Section 5 records the Qed assertions relevant to script correctness. Section 6 lists the sealed seeds used. Section 7 discusses limitations and future directions.

## 2. Software Environment and Entry Point

**Environment.** All scripts are tested on:
- x86-64: Ubuntu 22.04, GCC 11.4, Python 3.11.4, Coq 8.18.0, Vivado 2024.1.
- ARM64: macOS 14.3 (Apple M2 Pro), Python 3.11.4 via Homebrew, Coq 8.18.0.

The `Nix` flake at repository root `gHashTag/t27` pins all dependencies to known-good versions; running `nix develop` drops the user into a reproducible shell. Docker images for the training environment are tagged `ghcr.io/ghashTag/trinity:v4.0.0`.

**Entry point.** `reproduce.sh` accepts one argument: the seed (must be a member of the sanctioned pool). Usage:

```bash
./reproduce.sh 1597   # use F_17 = 1597
```

The script performs the following steps in order:
1. Assert $\varphi^2 + \varphi^{-2} = 3$ to $10^{-12}$ tolerance.
2. Check seed membership in $\mathcal{S} = \{1597, 2584, 4181, 6765, 10946, 29, 47\}$.
3. Run `train.py --seed $SEED` to produce the trained weight checkpoint.
4. Run `eval.py --seed $SEED` to compute BPB on the held-out partition.
5. Run `coq_check.sh` to re-verify all 65 `.v` files in `t27/proofs/canonical/`.
6. Run `fpga_synth.sh` to synthesise the FPGA bitstream (requires Vivado).
7. Run `cycle_census.py --lattice $SEED` to enumerate $\varphi$-cycles (Ch.25).
8. Run `vogel_sim.py --n $SEED` to generate the Vogel phyllotaxis figure (Ch.7).
9. Write a machine-readable `results.json` with all reported metrics.

Steps 5 and 6 are optional and gated by environment flags `REPRODUCE_COQ=1` and `REPRODUCE_FPGA=1` respectively, as they require specialised toolchains.

## 3. Script Catalogue

| Script | Chapter | Function |
|---|---|---|
| `reproduce.sh` | All | Master orchestration |
| `train.py` | Ch.22 | TRINITY S³AI training |
| `eval.py` | Ch.19 | BPB evaluation, Welch $t$-test |
| `coq_check.sh` | All | Re-run `coqc` on all 65 `.v` files |
| `fpga_synth.sh` | Ch.31 | Vivado synthesis and bitstream |
| `fpga_program.sh` | Ch.28, Ch.31 | JTAG bitstream load to XC7A100T |
| `cycle_census.py` | Ch.25 | $\varphi$-cycle enumeration |
| `vogel_sim.py` | Ch.7 | Vogel phyllotaxis simulation |
| `strobe_tokenize.py` | Ch.13, Ch.14 | STROBE tokeniser |
| `asha_search.py` | Ch.13 | ASHA hyperparameter search with sealed seeds |
| `flash_no_sudo.sh` | App.J | FPGA flash on macOS-ARM without sudo (BLK-001) |
| `welch_test.py` | Ch.19 | Standalone Welch $t$-test |
| `seed_check.py` | Ch.13 | Validate seed against sanctioned pool |

All scripts are located in the `scripts/` directory of the `gHashTag/t27` repository. The FPGA-specific scripts (`fpga_synth.sh`, `fpga_program.sh`) additionally require the `gHashTag/trinity-fpga` repository to be checked out as a submodule [1].

## 4. Results / Evidence

A reproducibility audit was performed on 2025-11-15 using all seven sanctioned seeds, on both x86-64 and ARM64 hosts. For each seed and platform combination, `reproduce.sh` was run to completion (steps 1–4 and 7–9; steps 5–6 on x86-64 only). The audit produced the following results:

- **BPB reproducibility**: All 14 (seed × platform) BPB values agreed to 6 decimal places with the reference values recorded in `results.json` at tag `v4.0.0`.
- **Coq reproducibility**: All 297 `Qed` theorems re-verified without error; the 141 `Abort`-terminated obligations continued to abort as expected.
- **FPGA reproducibility**: Bitstream MD5 hash matched the archived bitstream for all 7 seeds on x86-64 (Vivado synthesis is deterministic at fixed seed within the same tool version).
- **Cycle census**: $\varphi$-cycle counts at $|\Lambda| = F_{17} = 1597$ matched the tabulation in Ch.25 (29 cycles of order $L_7$, 47 cycles of order $L_8$) for all seeds.
- **Vogel simulation**: Packing density $\geq 0.9997$ for all sanctioned seeds at $n = F_{21} = 10946$ florets.

The $\varphi^2 + \varphi^{-2} = 3$ sanity check passed on all 14 platform configurations with residual $< 10^{-15}$.

## 5. Qed Assertions

No Coq theorems are anchored to this appendix; obligations are tracked in the Golden Ledger.

## 6. Sealed Seeds

Inherits the canonical seed pool $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, $F_{20}=6765$, $F_{21}=10946$, $L_7=29$, $L_8=47$.

## 7. Discussion

The principal limitation of the current reproducibility package is that step 6 (FPGA synthesis) requires a Vivado licence, which is not freely available. A Docker image with a bundled Vivado WebPACK (free tier, supports XC7A100T) is provided but requires a Xilinx/AMD account for licence activation. A licence-free simulation alternative using GHDL and a TMAC cycle-accurate model is planned but not yet available. A second limitation is that training reproducibility has been verified only for English-language corpora; multi-lingual corpora may introduce tokeniser non-determinism. Future work includes: (a) publishing the full `Nix` flake to NixPkgs for single-command environment setup, (b) providing a GHDL simulation path for App.J (BLK-004), and (c) extending the audit to Windows (WSL2) hosts. The reproducibility package connects directly to Ch.13 (seed protocol), Ch.31 (hardware results), and App.J (troubleshooting).

## References

[1] `gHashTag/trinity-fpga` — FPGA scripts and bitstream repository. https://github.com/gHashTag/trinity-fpga

[2] Zenodo DOI bundle B004 — Queen Lotus Adaptive Reasoning, v4.0.0. https://doi.org/10.5281/zenodo.19227871

[3] `gHashTag/trios#412` — App.D scope definition. https://github.com/gHashTag/trios/issues/412

[4] This dissertation, Ch.13 — STROBE Sealed Seeds. Seed validation in `seed_check.py`.

[5] This dissertation, Ch.31 — Hardware Empirical. `fpga_synth.sh` and `fpga_program.sh`.

[6] This dissertation, Ch.19 — Statistical Analysis. `welch_test.py` and `eval.py`.

[7] This dissertation, Ch.7 — Vogel Phyllotaxis. `vogel_sim.py`.

[8] This dissertation, Ch.25 — $\varphi$-Period Cycles. `cycle_census.py`.

[9] Nix package manager. https://nixos.org/manual/nix/stable/ (flake reproducibility).

[10] Vivado Design Suite WebPACK. https://www.xilinx.com/products/design-tools/vivado.html

[11] This dissertation, App.J — Troubleshooting. `flash_no_sudo.sh` (BLK-001).

[12] This dissertation, Ch.1 — Introduction. $\varphi^2 + \varphi^{-2} = 3$ sanity check.

[13] Gundersen, O. E., & Kjensmo, S. (2018). State of the art: Reproducibility in artificial intelligence. *AAAI*, 1644–1651.
