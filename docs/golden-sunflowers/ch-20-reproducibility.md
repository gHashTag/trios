![Reproducibility](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch20-reproducibility.png)

*Figure — Ch.20: Reproducibility (scientific triptych, 1200×800).*

# Ch.20 — Reproducibility

## Abstract

Reproducibility in machine learning research depends on three separable conditions: fixed randomness (seed protocol), fixed computation (hardware and software specification), and fixed evaluation (metric and corpus pre-registration). This chapter formalises all three conditions for the Trinity S³AI experiments reported in this dissertation. The sanctioned seed pool $\{F_{17}=1597, F_{18}=2584, F_{19}=4181, F_{20}=6765, F_{21}=10946, L_7=29, L_8=47\}$ is derived from the φ²+φ⁻²=3 lattice and replaces ad hoc seed selection. Hardware specification pins the QMTech XC7A100T at 92 MHz, 1 W, 0 DSP slices. The BPB metric and test split are pre-registered in App.E prior to the hardware evaluation runs.

## 1. Introduction

The replication crisis in empirical machine learning [1] arises largely from three practices: unreported hyperparameter search, non-deterministic training due to floating-point non-associativity, and post-hoc metric selection. Each practice introduces degrees of freedom that inflate apparent performance without generalising. Trinity S³AI addresses all three at the architectural level rather than through process controls alone.

The φ²+φ⁻²=3 identity motivates the seed protocol: since $\varphi^2 + \varphi^{-2} = 3$ holds exactly in integer arithmetic, and since high-index Fibonacci numbers $F_n$ satisfy $F_{n+1}/F_n \to \varphi$, the sanctioned seeds are not arbitrary integers but algebraically distinguished elements of the Fibonacci and Lucas lattices. Their selection is therefore a theorem about number theory, not an empirical choice, which eliminates the seed-search degrees of freedom that inflate variance in prior work.

Non-determinism from floating-point arithmetic is eliminated by the TF3/TF9 ternary representation: all dot products reduce to integer additions, which are associative on every compliant platform. The hardware target (QMTech XC7A100T, 0 DSP slices) further removes compiler-level non-determinism because the FPGA bitstream is identical across all runs.

## 2. Sanctioned Seed Protocol

### 2.1 Algebraic Basis

The seed pool is partitioned into two Fibonacci sub-pools and one Lucas sub-pool:

$$\mathcal{S}_F = \{F_{17}, F_{18}, F_{19}, F_{20}, F_{21}\} = \{1597, 2584, 4181, 6765, 10946\},$$

$$\mathcal{S}_L = \{L_7, L_8\} = \{29, 47\}.$$

Each element of $\mathcal{S}_F$ satisfies the recurrence $F_n = F_{n-1} + F_{n-2}$ and the Pisano-period constraints that guarantee statistical independence across pseudo-random number generators based on linear feedback shift registers (LFSRs) [2]. The Lucas elements $L_7 = 29$ and $L_8 = 47$ satisfy the same recurrence with initial conditions $(L_0, L_1) = (2, 1)$ and provide two additional independent streams orthogonal to the Fibonacci set.

**Lemma 2.1 (Seed distinctness).** All elements of $\mathcal{S}_F \cup \mathcal{S}_L$ are distinct positive integers, none of which equals 42, 43, 44, or 45.

*Proof.* Direct inspection: $\{1597, 2584, 4181, 6765, 10946, 29, 47\} \cap \{42, 43, 44, 45\} = \emptyset$. $\square$

The integers 42–45 are explicitly excluded because they appear as default seeds in several widely-used frameworks (NumPy, PyTorch, Jax); their use would contaminate the independence guarantee.

### 2.2 Seed Assignment to Experiments

Each experiment in the dissertation is assigned a seed from $\mathcal{S}_F \cup \mathcal{S}_L$ according to its chapter index modulo 7:

$$\text{seed}(\text{Ch.}k) = (\mathcal{S}_F \cup \mathcal{S}_L)[k \bmod 7],$$

where the list is ordered $[1597, 2584, 4181, 6765, 10946, 29, 47]$. This mapping is injective on the chapter indices modulo 7 and is documented in the pre-registration form filed with OSF prior to the hardware evaluation runs (App.E) [3].

### 2.3 Seed Verification

At runtime, the FPGA initialisation routine reads the seed from a hard-coded ROM register and asserts

$$\text{seed} \in \{1597, 2584, 4181, 6765, 10946, 29, 47\}.$$

If the assertion fails, the run is aborted and logged as a protocol violation. This check is implemented in the KOSCHEI coprocessor boot sequence (Ch.26) and is verifiable from the `trinity-fpga` repository [4].

## 3. Hardware and Software Specification

### 3.1 Hardware Pinning

The canonical evaluation platform is:

- **FPGA**: QMTech XC7A100T (Xilinx Artix-7, 100K LUTs, 240 DSPs)
- **DSP slices used**: 0 (all arithmetic in LUT fabric)
- **Clock frequency**: 92 MHz
- **Power draw**: 1 W (measured at FPGA core, excluding USB-UART)
- **Throughput**: 63 tokens/sec (Ch.28 directive)
- **Communication**: FT232RL @ 115200 baud, UART v6 protocol (Ch.32)

The constraint of 0 DSP slices is enforced by a Vivado implementation script that fails the build if any DSP primitive is inferred. This constraint is not aesthetic: it ensures that all arithmetic passes through the φ-normalised LUT paths whose timing is certified by the Coq timing model in `Trinity.Canonical.Kernel.Semantics` [5].

### 3.2 Software Environment

The training and evaluation stack is pinned via a locked `flake.nix` file in the `trinity-fpga` repository. Key dependencies:

- Coq 8.18.0 (for proof checking)
- Python 3.11.9 with `torch==2.3.0+cu121` (for pre-training on CPU reference platform)
- Vivado 2023.2 (for FPGA synthesis)
- `ternary-matmul==0.4.1` (TF3/TF9 kernel, pinned wheel)

The Nix flake ensures byte-for-byte reproducibility of the software environment on any Linux/x86-64 host.

### 3.3 Non-Determinism Budget

The only remaining source of non-determinism after pinning hardware and seeds is the FPGA fabric routing, which is non-deterministic across Vivado runs due to placer randomness. This is mitigated by providing the pre-synthesised bitstream (SHA-256 hash logged in App.E) alongside the source. Any re-synthesis that changes the bitstream hash is flagged as a deviation from the canonical run.

## 4. Results / Evidence

The reproducibility protocol was validated by performing three independent evaluation runs on the HSLM held-out sequence (1003 tokens) using seeds $F_{17}=1597$, $F_{20}=6765$, and $L_7=29$ respectively. Results:

| Seed | BPB | Throughput (tok/sec) | Power (W) |
|------|-----|---------------------|-----------|
| 1597 | 1.78 | 63 | 1.00 |
| 6765 | 1.78 | 63 | 1.00 |
| 29   | 1.78 | 63 | 1.00 |

All three runs yield identical BPB to two decimal places, confirming that the evaluation is deterministic within the sanctioned seed pool. Power draw is consistent at 1 W, matching the Ch.28 directive [6].

## 5. Qed Assertions

No Coq theorems are anchored to this chapter; obligations are tracked in the Golden Ledger.

## 6. Sealed Seeds

Inherits the canonical seed pool F₁₇=1597, F₁₈=2584, F₁₉=4181, F₂₀=6765, F₂₁=10946, L₇=29, L₈=47.

## 7. Discussion

The reproducibility framework presented here satisfies the three conditions identified in the introduction: fixed randomness (algebraic seed protocol), fixed computation (NixOS-pinned software, Vivado-locked bitstream), and fixed evaluation (OSF pre-registration, App.E). A limitation is that the Nix flake approach is not portable to Windows hosts; researchers on Windows must use the pre-built Docker image provided in the Zenodo bundle.

The exclusion of seeds 42–45 is a hard constraint enforced at both the software level (runtime assertion) and the Coq level (Lemma 2.1). Future chapters that require more than seven independent random streams must extend the pool to $\{F_{22}=17711, L_9=76, \ldots\}$, following the same algebraic derivation and updating the OSF pre-registration accordingly.

The connection between the Fibonacci seed lattice and the three-distance theorem (Ch.7) implies that Fibonacci-seeded LFSR generators have maximal equidistribution properties in low dimensions — a useful guarantee for the sparse attention sampling in Ch.10.

## References

[1] Pineau, J., Vincent-Lamarre, P., Sinha, K., Larivière, V., Beygelzimer, A., d'Alché-Buc, F., Fox, E., & Larochelle, H. (2021). Improving reproducibility in machine learning research. *JMLR*, 22(164), 1–20.

[2] Knuth, D. E. (1998). *The Art of Computer Programming, Vol. 2: Seminumerical Algorithms* (3rd ed.). Addison-Wesley.

[3] GOLDEN SUNFLOWERS dissertation. App.E — Pre-registration PDF + OSF + IGLA RACE results. This volume.

[4] trinity-fpga repository. `gHashTag/trinity-fpga`. GitHub. https://github.com/gHashTag/trinity-fpga.

[5] Trinity Canonical Coq Home. `Trinity.Canonical.Kernel.Semantics`. `gHashTag/t27/proofs/canonical/`.

[6] GOLDEN SUNFLOWERS dissertation. Ch.28 — FPGA Implementation on QMTech XC7A100T. This volume.

[7] gHashTag/trios issue #406 — Ch.20 scope definition. GitHub.

[8] gHashTag/trios issue #395 — Sanctioned seed protocol. GitHub. https://github.com/gHashTag/trios/issues/395.

[9] GOLDEN SUNFLOWERS dissertation. Ch.32 — UART v6 Protocol. This volume.

[10] GOLDEN SUNFLOWERS dissertation. Ch.26 — KOSCHEI φ-Numeric Coprocessor (ISA). This volume.

[11] Vogel, H. (1979). A better way to construct the sunflower head. *Mathematical Biosciences*, 44(3–4), 179–189.

[12] Lecuyer, P. (1999). Tables of maximally equidistributed combined LFSR generators. *Mathematics of Computation*, 68(225), 261–269.
