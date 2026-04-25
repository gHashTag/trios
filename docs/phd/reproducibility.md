# Reproducibility Manifest — Flos Aureus

ACM Artifact Evaluation badge targets: **Functional · Reusable · Available**.

This manifest is the single point of entry for a reviewer reproducing the
empirical results of the monograph. There is no `.sh` wrapper anywhere in
the pipeline (R1).

## Hardware profile

| Component | Specification |
|-----------|---------------|
| CPU       | x86_64, ≥ 8 cores, AVX-512 (any 2023+ Xeon / EPYC / Ryzen). |
| GPU       | NVIDIA, CUDA 12.1+, ≥ 24 GiB VRAM. |
| RAM       | ≥ 32 GiB. |
| Disk      | ≥ 50 GiB free for caches and CSV outputs. |

## Software dependencies

| Component        | Version              | Source |
|------------------|----------------------|--------|
| Rust toolchain   | stable, pinned in `rust-toolchain.toml` | rustup |
| `tectonic` crate | 0.15                 | crates.io |
| Coq              | 8.18                 | opam   |
| `biber`          | bundled with tectonic | crates.io |

There are no Python, Bash, or Make dependencies (R1).

## Entry points

| Goal                            | Command |
|---------------------------------|---------|
| Build the monograph PDF         | `cargo run -p trios-phd -- compile` |
| Audit the monograph             | `cargo run -p trios-phd -- audit` |
| Audit a single chapter          | `cargo run -p trios-phd -- audit --chapter 24` |
| Audit falsification scaffolding | `cargo run -p trios-phd -- audit --falsification` |
| Audit bibliography balance      | `cargo run -p trios-phd -- audit --bibliography` |
| Audit page-count budget         | `cargo run -p trios-phd -- audit --pagecount` |
| Regenerate Coq citation map     | `cargo run -p trios-phd -- coq-map` |
| Reproduce a chapter             | `cargo run -p trios-phd -- reproduce --chapter 24` |
| Build defense package           | `cargo run -p trios-phd -- defense build` |

## Seeds

All empirical chapters report on the canonical seed set:

```
SEEDS = { 17, 42, 1729 }
```

A reviewer who passes `--seeds 17,42,1729` to `reproduce` should recover
the reported numbers within ±0.5 %. Wider tolerance is automatically
flagged as a corroboration failure in Appendix B.

## Data integrity

Every CSV consumed by the reproduction binary is hashed (SHA-256) and the
hash is verified at load time against this manifest:

| Chapter | CSV path                                  | SHA-256 |
|---------|-------------------------------------------|---------|
| 24      | `data/chapter-24/bpb_seeds.csv`           | (filled at submission) |
| 25      | `data/chapter-25/asha_survival.csv`       | (filled at submission) |
| 26      | `data/chapter-26/gf16_floor.csv`          | (filled at submission) |
| 28      | `data/chapter-28/ablations.csv`           | (filled at submission) |

Hashes are filled in by `cargo run -p trios-phd -- reproduce --freeze`,
which is the final pre-submission step.

## Licensing

| Asset            | License     |
|------------------|-------------|
| Rust source      | MIT         |
| Coq proofs       | Apache-2.0  |
| Monograph text   | CC-BY-4.0   |
| Figures (own)    | CC-BY-4.0   |

## Permanent identifiers

| Object                      | DOI / URL |
|-----------------------------|-----------|
| Trinity paper anchor        | <https://doi.org/10.5281/zenodo.19227877> |
| Companion deposit (origin)  | <https://doi.org/10.5281/zenodo.18947017> |
| Companion deposit (proofs)  | <https://doi.org/10.5281/zenodo.19227879> |
| Source repository           | <https://github.com/gHashTag/trios>, tag `phd/v1.0` |

## Contact

Issues and questions: <https://github.com/gHashTag/trios/issues>.
