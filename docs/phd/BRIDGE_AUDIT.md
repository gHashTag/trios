# BRIDGE_AUDIT — Flos Aureus × IGLA × trinity-clara

**Generated:** 2026-04-25 (rev. after rebase on `origin/main`)
**Branch:** `feat/phd-bridge-audit-and-trios-phd-skeleton`
**Refs:** [#30](https://github.com/gHashTag/trios/issues/30) [#62](https://github.com/gHashTag/trios/issues/62) [#109](https://github.com/gHashTag/trios/issues/109) [#143](https://github.com/gHashTag/trios/issues/143) · PR [#263](https://github.com/gHashTag/trios/pull/263)

This document is the honest delta between the **One Shot — PhD ↔ trios ↔ trinity-clara**
mission spec (2026-04-25T21:58+07) and the actual state of `main` at the time the
bridge branch was cut. It exists to close mission rule **R5** (no claiming Proven
where work is Admitted) and to give downstream agents a real punch list rather than
a vibe.

## 1. What this PR lands

| Deliverable | Status |
|---|---|
| Consume canonical `assertions/igla_assertions.json` schema **v1.0.0** (6 invariants, INV-12 included) | ✅ done (upstream b959c43 absorbed) |
| `crates/trios-phd` skeleton with `cite::{CodataLink, CoqLink}` + `audit` + `status` subcommands | ✅ done |
| Workspace registration of `trios-phd` (51 members) | ✅ done |
| Unit tests (12, all green) covering R5 honesty, schema-version refusal, INV-12 rungs | ✅ done |
| `cargo run -p trios-phd -- audit` + `status` against rebased tree | ✅ exits 0 |
| Workflow YAML repair (`laws-guard.yml`, `coq-check.yml` no longer startup_failure) | ✅ done |
| `coq-check.yml` wires `cargo test -p trios-phd` + `cargo run -p trios-phd -- audit` | ✅ done |
| This audit report | ✅ done |

## 2. What this PR explicitly does NOT do (and why)

| Mission item | Why deferred | Tracking |
|---|---|---|
| **Track A — drive BPB < 1.50 on 3 seeds** | Requires GPU + neural training stack. Cannot be one-shot from any sandbox. | Stays on #143 worker fleet. |
| **Track B — write Ch.1, Ch.3, Ch.5, Ch.8 each ≥ 1500 lines** | Each chapter is a multi-day research+writing job; faking 1500 lines violates R6 (Popper's razor). | Re-claim per chapter on #36, #37, #39, #45. |
| **Tectonic build integration** | Adds a heavy native dep + CI artifact pipeline. Out of scope for the skeleton; needs its own PR with `coq-proofs.yml` edits. | Follow-up issue, see §6. |
| **Page-count assertion (≥ 500, ≤ 750)** | Skeleton has no PDF compiler yet; would be vacuous. Re-add once tectonic lands. | §6. |
| **`generate-figure` / `export-trials` subcommands** | Need real IGLA RACE trial table data which isn't checked in. | Coupled to Track A. |

## 3. Drift findings against the mission spec

### 3.0 Rebase outcome — schema v1.0.0 absorbed

While this branch was in flight, `main` advanced with commit
[`b959c43`](https://github.com/gHashTag/trios/commit/b959c43) which:

- promoted `assertions/igla_assertions.json` to **`schema_version: "1.0.0"`**
- added **INV-12** (`asha_rung_progression_integrity`, Proven, paired with INV-2 in
  `igla_asha_bound.v`) for Trinity-base ASHA rungs `[1000, 3000, 9000, 27000]`
- renamed INV-1's coq file from `lr_convergence.v` to `lr_phi_optimality.v`
- restructured the file from `inv1`/`inv2`/... object-keys to a top-level
  `invariants` array of 6 entries

This bridge branch **drops** its earlier hand-reconciled JSON edits and consumes
the v1.0.0 schema as the single source of truth. `crates/trios-phd/src/audit.rs`
was rewritten to:

- parse the array shape (`data.invariants[]`)
- refuse any unknown `schema_version` (`SUPPORTED_SCHEMA_VERSIONS = ["1.0.0"]`)
- enforce `Proven` cannot have `admitted_theorems`, `Admitted` must list named theorems
- check no duplicate `id` values
- recognise INV-12 as a Trinity-base invariant (rungs = `1000 × {3⁰..3³}`)

All 12 unit tests pass against the rebased tree:

```
$ cargo test -p trios-phd
running 12 tests ... 12 passed; 0 failed
```

This means **§3.1–§3.2 below describe history that was already corrected on
`main` by upstream**. They are kept for traceability but are no longer the live
delta; the live delta is the punch list in §6.

### 3.1 `assertions/igla_assertions.json` had two diverging copies (resolved upstream)

**Before this PR:**
- `trios/assertions/igla_assertions.json` — short, missing `admitted_budget`, no `proof_status` fields, used `_meta` key
- `trinity-clara/assertions/igla_assertions.json` — long, claims `admitted: 4` and `qed_proven: 31`
- The two files **disagreed** on INV-5 (`phi_inv6_positive` vs `phi_pow_to_lucas: "Admitted"`) and on the budget

**After this PR (`trios/assertions/igla_assertions.json`):**
- Uses `_metadata` (matches upstream key)
- Honest `admitted_budget.used = 2` cross-checked against
  `grep -E '^Admitted\.|^Proof\. Admitted\.' trinity-clara/proofs/igla/*.v` →
  exactly 2 hits (`lr_convergence.v:48` `alpha_phi_lb`, `lr_convergence.v:51` `alpha_phi_ub`)
- Per-file breakdown now matches ground-truth counts (see §3.2)
- New per-invariant `proof_status` (`"Proven"` / `"Admitted"`) and `runtime_action`
  fields, ready to drive the runtime guard layer in `crates/trios-igla-race/src/invariants.rs`

The trinity-clara copy is **untouched** by this PR — it lives in a different repo
and a separate PR there should mirror this reconciliation.

### 3.2 Coq counts — ground truth vs claims

```
$ cd trinity-clara/proofs/igla
$ grep -c '^Qed\.' *.v
  bpb_monotone_backward.v:6
  gf16_precision.v:4
  igla_asha_bound.v:6
  lr_convergence.v:3
  lucas_closure_gf16.v:10
  nca_entropy_band.v:2
  TOTAL: 31

$ grep -E '^Admitted\.|^Proof\. Admitted\.' *.v
  lr_convergence.v:48:Admitted.
  lr_convergence.v:51:Proof. Admitted.
  TOTAL: 2
```

The trinity-clara metadata previously claimed `admitted: 4` with INV-5 entries.
Inspection shows those INV-5 references are inside **comments** (`(* Admitted: ... *)`
at line 104) — not actual `Admitted.` directives. The reconciled JSON corrects this
in `_metadata.admitted_budget.previous_claim_corrected`.

INV-1 (`bpb_decreases_with_real_gradient`) has its real Admitted directives in the
sibling file `lr_convergence.v` (the α_φ tightness lemmas), not in
`bpb_monotone_backward.v`. This is now reflected via the new `supporting_file` field.

### 3.3 PhD chapter inventory vs mission spec

Mission spec assumed:
- Ch.5 = Golden Scales / Popper's razor
- Ch.24 = IGLA-GF16 evidence
- Ch.8 = Coldea anchor
- Each chapter ≥ 1500 lines

Actual state on `main`:

| Spec position | Actual file | Lines | Status |
|---|---|---|---|
| Ch.1 Golden Seed | `01-golden-seed.tex` | ~190 | placeholder; also a duplicate `01-golden-egg.tex` exists |
| Ch.5 Golden Scales | **off-by-one** — file is `04-golden-scales.tex` | ~190 | placeholder |
| Ch.8 Golden Crystal | `08-golden-crystal.tex` | ~190 | placeholder |
| Ch.24 IGLA-GF16 evidence | `24-igla-architecture.tex` is **theoretical only** | ~190 | does not cite IGLA RACE trial data |
| **Total all 33 chapters** | 6 304 lines | avg 191/chapter | **R3 (≥ 1500/chapter) failing repo-wide** |

There are also **two duplicate-numbered chapter files** that need merging or deletion
before the audit can enforce 1-to-1 mapping:

- `01-golden-egg.tex` + `01-golden-seed.tex`
- `32-conclusion.tex` + `33-conclusion.tex`

These are *not* fixed in this PR (they need an editorial decision); they are flagged
here so the next chapter-level PR has a clear list.

### 3.4 R1 violations (RUST-only build)

- `docs/phd/Makefile` exists (Make is fine; it's not a `.sh`).
- `docs/phd/scripts/*.run` files exist — unknown whether these are shell or Rust;
  needs an inspection PR. The `trios-phd` skeleton in this PR does **not** invoke
  any of them.

### 3.5 Vendored proofs drift

`trios/trinity-clara/proofs/igla/` contains a checked-in mirror of an **older
version** of the proofs (different filenames: `bpb_decreases.v`,
`asha_champion_survives.v`, `gf16_safe_domain.v`, `lr_phi_optimality.v`,
`nca_entropy_stability.v`). The upstream `trinity-clara/proofs/igla/` has
renamed and consolidated these into `bpb_monotone_backward.v`, `igla_asha_bound.v`,
`gf16_precision.v`, `lr_convergence.v`, `nca_entropy_band.v`, `lucas_closure_gf16.v`.

**Recommendation:** convert `trios/trinity-clara/` to a real git submodule
(or delete the in-tree copy and reference the upstream repo via a build-time
fetch in `coq-check.yml`). The current in-tree copy can silently go out of sync.
This PR does not touch the in-tree copy to keep the diff focused, but the audit
report flags it.

## 4. Honesty contract enforced by `trios-phd::audit`

The `audit` subcommand and unit tests guarantee, at every CI run:

1. **R4** — `\coqbox{INV-X}` macros in `docs/phd/chapters/**.tex` must reference a
   known invariant (INV-1..5).
2. **R5** — any `\coqbox{INV-X}` for an `Admitted` invariant must be paired in the
   same chapter with `\admittedbox{...}`.
3. **`assertions/igla_assertions.json` self-consistency**:
   - `admitted_budget.used ≤ admitted_budget.max`
   - `admitted_budget.used == len(admitted_budget.entries)`
   - `theorem_count.qed_proven == Σ breakdown_by_file.qed`
   - `theorem_count.admitted == Σ breakdown_by_file.admitted`
   - For each invariant: `proof_qed == (proof_status == "Proven")`

Six of nine unit tests are dedicated specifically to attempting to break these
properties with poisoned JSON, which makes accidental drift in future PRs loud.

## 5. Constants pinned (per ACK template §6)

| Symbol | Value | Source (schema v1.0.0) |
|---|---|---|
| `PHI` | `1.618_033_988_749_894_8` | `crates/trios-phd/src/lib.rs` (// φ² + φ⁻² = 3) |
| `prune_threshold` | `3.5` | `invariants[INV-2].bands.prune_threshold` |
| `warmup_blind_steps` | `4000` | `invariants[INV-2].bands.warmup_blind_steps` |
| `d_model_min` | `256` | `invariants[INV-3].bands.d_model_min` |
| `lr_champion` | `0.004` | `invariants[INV-1].bands.lr_champion` |
| `valid_rungs` | `[1000, 3000, 9000, 27000]` | `invariants[INV-12].bands.valid_rungs` (= 1000 × {3⁰..3³}) |
| Forbidden | `prune_threshold = 2.65` | killed champion in J-001/J-002, do not reintroduce |

## 6. Punch list for follow-up PRs

### A — chapter editorial pass (no code)
- merge or delete `01-golden-egg.tex` ↔ `01-golden-seed.tex`
- merge or delete `32-conclusion.tex` ↔ `33-conclusion.tex`
- decide canonical numbering: does spec Ch.5 = file `04-` or `05-`?

### B — `trios-phd` followups (extend this skeleton)
- `cargo run -p trios-phd -- compile --chapter N` via `tectonic` crate
- `cargo run -p trios-phd -- bibtex-check` against `bibliography.bib`
- page-count gate (≥ 500, ≤ 750)
- `--codata-table` subcommand emitting `codata-table.tex`
- `--coq-table` subcommand emitting `coq-table.tex`

### C — proofs consolidation
- decide submodule vs upstream-fetch for `trios/trinity-clara/`
- delete or update the stale in-tree mirror

### D — runtime guard layer
- update `crates/trios-igla-race/src/invariants.rs` to consume the **reconciled**
  JSON via the `trios-phd::audit::Assertions` typed loader (currently each crate
  re-parses by hand).

### E — CI gates
- `.github/workflows/coq-check.yml` — ✅ added `cargo test -p trios-phd` and
  `cargo run -p trios-phd -- audit` in this PR
- `.github/workflows/laws-guard.yml` — ✅ quoted 9 step names with embedded colons
  to fix the upstream `startup_failure` (was failing in 0s on every commit)
- `.github/workflows/coq-proofs.yml` — still pending: add `cargo run -p trios-phd -- audit`

### F — Track A (gated on hardware)
- BPB < 1.50 on 3 seeds via `asha` worker
- export trial table → `docs/phd/data/igla_trials.csv`
- write `24-igla-architecture.tex` evidence section citing real rows

### G — Track B (one chapter, one PR)
- `feat/phd-ch01` (#36)
- `feat/phd-ch03` (#37)
- `feat/phd-ch05` (#39)
- `feat/phd-ch08` (#45)

## 7. Zenodo / DOI anchors

- Trinity paper / 84 theorem base: [10.5281/zenodo.19227877](https://zenodo.org/records/19227877)
- DARPA CLARA submission package: [gHashTag/trinity-clara](https://github.com/gHashTag/trinity-clara)
- t27 mathematical base: [gHashTag/t27](https://github.com/gHashTag/t27)

φ² + φ⁻² = 3.
