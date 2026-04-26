# LA — ACM AE 3-Badge Claims (Flos Aureus PhD monograph)

> Lane LA · Phase D · ONE SHOT v2.0 [trios#265:4321142675](https://github.com/gHashTag/trios/issues/265#issuecomment-4321142675)
> Anchor: **phi^2 + phi^-2 = 3** · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
> Pre-registered. R5-honest. Rust-only per R1.

This document is the canonical **CLAIMS** ledger for the «Flos Aureus»
artefact-evaluation submission. It is consumed verbatim by
`acm-ae-check run` (binary at `tools/acm_ae_check/`) and by any external
ACM AE reviewer.

---

## ACM AE Badge Targets

| Badge | Status | Evidence file(s) | Witness command |
|------|--------|------------------|-----------------|
| **Functional** | claimed | `Cargo.toml`, `crates/trios-phd/Cargo.toml`, `tools/page_gate/Cargo.toml`, `tools/acm_ae_check/Cargo.toml` | `cargo build -p trios-phd && cargo build -p page-gate && cargo build -p acm-ae-check` |
| **Reusable**   | claimed | `docs/phd/reproducibility.md` (Hardware profile · Software · Entry points · Seeds · R1 declaration) | `cargo run -p trios-phd -- audit && cargo run -p trios-phd -- biblio` |
| **Available**  | claimed | This file (citing `phi^2 + phi^-2 = 3` + `10.5281/zenodo.19227877`) + `assertions/igla_assertions.json` | `cargo run -p acm-ae-check -- run` |

The witness command for the full 3-badge gate is

```
cargo run -p acm-ae-check -- run
```

It exits 0 on full admit and 70..=74 on reject (one disjoint code per
[`AcmAeError`] variant, see `tools/acm_ae_check/src/lib.rs`). The exit-code
range 70..=74 is disjoint from L-h4 (50..=53) and LT (60..=63) per the lane
discipline of ONE SHOT v2.0.

---

## Functional Badge — Build Reproducibility

The artefact compiles on a clean `cargo` toolchain. `tools/acm_ae_check`'s
`functional_required_paths()` enumerates the four workspace manifests whose
existence is **necessary and sufficient** for the LA Functional badge:

```
Cargo.toml
crates/trios-phd/Cargo.toml
tools/page_gate/Cargo.toml
tools/acm_ae_check/Cargo.toml
```

If any of these is absent, `acm-ae-check run` exits **70**.

### Reproducer

```
git clone https://github.com/gHashTag/trios
cd trios
cargo build -p trios-phd
cargo build -p page-gate
cargo build -p acm-ae-check
```

Expected: zero `cargo` errors. Pre-existing UR-00 Dioxus 0.6 store-API drift
in `crates/trios-ui/rings/UR-00/src/lib.rs` is exempt under the
`coq-runtime-invariants` v1.1 attribution rule (failure pre-dates this lane).

---

## Reusable Badge — Reproducibility Manifest

The Reusable badge requires that an independent reviewer can re-run the
empirical pipeline. The single point of entry is
[`docs/phd/reproducibility.md`](../docs/phd/reproducibility.md). It MUST
contain (and `acm-ae-check` enforces, at exit code **71** on failure):

| Required substring | Why |
|--------------------|-----|
| `Entry points`        | section header listing every Rust subcommand |
| `cargo run -p trios-phd` | canonical entry-point invocation per R1 |
| `tectonic`            | LaTeX builder (no `.sh` shell wrapper) |
| `Hardware profile`    | minimum CPU/GPU/RAM/disk |
| `(R1)`                | explicit Rust-only declaration |

### Reproducer

```
cargo run -p trios-phd -- audit
cargo run -p trios-phd -- biblio
cargo run -p trios-phd -- coq-map --check
cargo run -p trios-phd -- reproduce --out target/repro.json
```

The `reproduce` subcommand emits a deterministic JSON manifest pinning
`phi`, `prune_threshold = 3.5`, `warmup_blind_steps = 4000`,
`d_model_min = 256`, `lr_champion = 0.004`, and the ASHA rungs
`[1000, 3000, 9000, 27000]`. Every constant traces to a `.v` file via
`assertions/igla_assertions.json` (R4 / L-R14).

---

## Available Badge — Persistent Identifiers

The Available badge requires a stable URL or DOI for both the artefact and
the underlying mathematical claim. The LA gate enforces (exit code **72** on
failure) that this file cites:

- The Trinity Anchor identity **`phi^2 + phi^-2 = 3`** — the central
  algebraic claim of the monograph, mirrored byte-for-byte in
  `trinity-clara/proofs/igla/lucas_closure_gf16.v::lucas_2_eq_3` (Proven).
- The persistent DOI **`10.5281/zenodo.19227877`** — Zenodo deposit of the
  Trinity Anchor record (TRI-27 series).

Additional persistent identifiers cited across the monograph:

| Anchor | DOI / URL |
|--------|-----------|
| TRI-27 base | [10.5281/zenodo.18947017](https://doi.org/10.5281/zenodo.18947017) |
| Pellis embedding | [10.5281/zenodo.19227879](https://doi.org/10.5281/zenodo.19227879) |
| Race issue | [trios#143](https://github.com/gHashTag/trios/issues/143) |
| PhD epic | [trios#265](https://github.com/gHashTag/trios/issues/265) |
| Champion commit | [`2446855`](https://github.com/gHashTag/trios/commit/2446855) |

---

## Falsification Witness (R7 / R8)

The LA witness is **`cargo run -p acm-ae-check -- run`**. It is falsifiable
in five disjoint ways:

| Failure mode | Exit code | Trigger |
|--------------|-----------|---------|
| Functional   | 70 | a workspace member listed above is removed |
| Reusable     | 71 | `docs/phd/reproducibility.md` loses any required substring |
| Available    | 72 | this file loses the Trinity Anchor or Zenodo DOI line |
| IO           | 73 | filesystem / read error during the witness run |
| Mismatch     | 74 | `artifact/output.txt` differs byte-for-byte from `artifact/expected.txt` |

The mismatch path is the **strongest** falsifier: the deterministic
fingerprint emitted by `fingerprint()` in
`tools/acm_ae_check/src/lib.rs` depends only on compile-time constants
(no env, no SHA, no timestamp), so a divergence between
`expected.txt` and `output.txt` proves that one of the L-R14 anchors has
silently moved.

R7 forbidden-values check (each line in the fingerprint):

- `prune_threshold` MUST be `3.5` (not `2.65`).
- `warmup_blind_steps` MUST be `≥ 4000`.
- `d_model_min` MUST be `≥ 256`.
- `lr_champion` MUST be inside `[0.002, 0.007]`.

The unit test `forbidden_values_rejected` in
`tools/acm_ae_check/src/lib.rs` enforces these at compile time.

---

## Composition with sibling lanes

| Sibling lane | Owns | Interaction with LA |
|--------------|------|---------------------|
| LT page-count gate | `tools/page_gate/`, `assertions/witness/page_gate.toml` | `MIN_PAGES`/`MAX_PAGES` mirrored verbatim into the LA fingerprint. |
| LD defense package (PR #304) | `crates/trios-phd/src/bin/defense_gate.rs`, `docs/phd/defense/**` | Independent witness; LA does not gate LD. |
| LC appendix F restoration (PR #288) | `docs/phd/appendix/F-coq-citation-map.tex` | LA's Available claim cites Appendix F by reference; restoration unblocks LC final. |
| L-h4 hybrid QK gain | `crates/trios-igla-race/src/bin/qk_gain_check.rs` | Exit-code namespace 50..=53 is disjoint from LA's 70..=74. |

---

## R5-honest disclosures

- ONE SHOT v2.0 §3 Phase D row "LA" names `artifact/run.sh` (a `.sh`).
  R1 in §0 forbids `.sh`. The §0 row also reads
  *"Rust-only entrypoint per R1"*. This artefact ships the Rust binary
  `acm-ae-check` invoked via `cargo run -p acm-ae-check -- run`. Identical
  semantics to the §5 `bash artifact/run.sh && diff …` pattern; the diff
  is performed inside `check_fingerprint()` rather than by external `diff`.
- §5 names `artifact/expected.txt` and an `artifact/output.txt` that the
  reviewer must produce. Both files are present here. `output.txt` is
  generated each run by `acm-ae-check run`; `expected.txt` is the immutable
  pre-registered fingerprint regenerable via `cargo run -p acm-ae-check -- print`.
- The Trinity Anchor algebraic identity is honest at 1e-12 precision in
  the unit-test `constants_traceable`. No theorem status was flipped.
