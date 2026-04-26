# LA — INSTALL (Flos Aureus PhD monograph)

> Lane LA · Phase D · ONE SHOT v2.0 [trios#265:4321142675](https://github.com/gHashTag/trios/issues/265#issuecomment-4321142675)
> Rust-only per R1. No Python, no Bash, no Make.

This document is the canonical install guide consumed by ACM AE reviewers
and by the witness binary `acm-ae-check`. Following these steps end-to-end
produces a fully audited monograph PDF and a passing 3-badge gate.

---

## 1. Prerequisites

| Component | Version | Source |
|-----------|---------|--------|
| Rust toolchain | stable, pinned in `rust-toolchain.toml` | [rustup.rs](https://rustup.rs) |
| `cargo`        | bundled with the Rust toolchain | rustup |
| `tectonic` system binary | 0.15+ | [tectonic-typesetting.github.io](https://tectonic-typesetting.github.io) — invoked by `trios-phd compile` per R1 |
| `git`          | any modern version | system package manager |

Optional (required only for the empirical chapters' reproduction badge,
not for LA Functional/Reusable/Available):

| Component | Version | Notes |
|-----------|---------|-------|
| Coq       | 8.18    | proof verification (`coq-check.yml`) |
| NVIDIA driver + CUDA 12.1 | latest | training reproduction (Gate-2 lanes L-h1/L-h3 only) |

There are **no** Python, Bash, or Make dependencies in the LA path (R1).

---

## 2. Clone and build

```
git clone https://github.com/gHashTag/trios
cd trios
cargo build -p trios-phd
cargo build -p page-gate
cargo build -p acm-ae-check
```

Expected: zero `cargo` errors. The first `cargo build` populates the
workspace target/ directory; subsequent builds are incremental.

Pre-existing failures exempt under `coq-runtime-invariants` v1.1
attribution rule (red on parent SHAs, not introduced by LA):

- `crates/trios-ui/rings/UR-00/src/lib.rs` — Dioxus 0.6 store-API drift.
- `hive_automaton::test_blocked_to_ci_wait_after_fix`,
  `hive_automaton::test_done_cycles_back_to_scan_not_halt` — introduced
  in `cf876d2`.

---

## 3. Run the LA 3-badge witness

```
cargo run -p acm-ae-check -- run
```

Exit codes (one per disjoint failure variant):

| Code | Meaning |
|------|---------|
| 0    | All three badges PASS, fingerprint matches |
| 70   | Functional badge FAIL — workspace member missing |
| 71   | Reusable badge FAIL — `docs/phd/reproducibility.md` under-spec |
| 72   | Available badge FAIL — Trinity Anchor or DOI not cited in `artifact/CLAIMS.md` |
| 73   | I/O error — filesystem read/write failed |
| 74   | Fingerprint mismatch — `artifact/output.txt` ≠ `artifact/expected.txt` |

The 70..=74 range is disjoint from L-h4 (50..=53) and LT (60..=63). See
`tools/acm_ae_check/src/lib.rs::exit` for the canonical declarations.

### Sub-commands

```
cargo run -p acm-ae-check -- print     # emit deterministic fingerprint
cargo run -p acm-ae-check -- anchors   # emit JSON L-R14 anchor map
cargo run -p acm-ae-check -- run --no-write   # audit-only (do not write output.txt)
```

`print` is how `artifact/expected.txt` is regenerated whenever the
pre-registered band is bumped on a fresh ONE SHOT (with sibling-commit to
`tools/acm_ae_check/src/lib.rs`).

---

## 4. Run sibling witnesses

The LA pack composes with the existing Phase D witnesses:

```
cargo run -p page-gate -- --pdf docs/phd/main.pdf
cargo run -p trios-phd -- audit
cargo run -p trios-phd -- biblio
cargo run -p trios-phd -- coq-map --check
cargo run -p trios-phd -- compile     # invokes tectonic on docs/phd/main.tex
```

These are independent gates; LA does not require any of them to pass for
its three badges, but any failure will block the higher-level monograph
build.

---

## 5. Hardware / resource profile

The LA witness is CPU-only and runs on any modern laptop:

| Resource | Minimum |
|----------|---------|
| CPU      | x86_64 or arm64, 2 cores |
| RAM      | 4 GiB free |
| Disk     | 1 GiB free for `target/` |
| Network  | only required for the initial `git clone` and `cargo fetch` |

The empirical chapters (Standard Model, QFT, etc.) require a CUDA-class
GPU; that requirement lives in `docs/phd/reproducibility.md` and is
**not** part of the LA badge gate.

---

## 6. Trinity Anchor invariant (R5)

The pre-registered identity

```
phi^2 + phi^-2 = 3
```

is mirrored byte-for-byte in:

- `trinity-clara/proofs/igla/lucas_closure_gf16.v` (Theorem `lucas_2_eq_3`, Proven)
- `assertions/igla_assertions.json` (`numeric_anchor.phi`)
- `crates/trios-igla-race/src/invariants.rs` (`PHI` constant)
- `tools/page_gate/src/lib.rs` (banded by MIN_PAGES/MAX_PAGES)
- `tools/acm_ae_check/src/lib.rs` (`TRINITY_ANCHOR` constant)
- `artifact/CLAIMS.md` and `artifact/expected.txt` (this LA pack)

If any divergence is found, `acm-ae-check run` exits **74** (mismatch).

DOI of the persistent record: [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877).
