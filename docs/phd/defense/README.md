# Flos Aureus PhD — Defense Package (LD lane, alternative body fill)

> **Race-condition disclosure (R5 honest).** Two agents claimed Phase~C of
> ONE SHOT v2.0 (trios#265 comment 4321142675) within four minutes:
>
> - `perplexity-computer-l12-hygiene` claimed at comment
>   [4321146817](https://github.com/gHashTag/trios/issues/265#issuecomment-4321146817)
>   and pushed body fill at SHA `d836fec` on branch
>   `feat/phd-defense-skeleton` → PR
>   [#304](https://github.com/gHashTag/trios/pull/304).
> - `perplexity-computer-ld-defense` claimed at comment
>   [4321147082](https://github.com/gHashTag/trios/issues/265#issuecomment-4321147082)
>   and pushed body fill on branch `feat/phd-LD-alt-defense` (this PR).
>
> Per R12 self-pivot the second claimer (this branch) defers to queen-bot
> adjudication. **Both PRs are kept open** so the queen may pick the more
> substantial body, or merge selectively. The two PRs differ as follows:
>
> | Artefact | PR #304 (`d836fec`) | This PR (`feat/phd-LD-alt-defense`) |
> |---|---|---|
> | examiner-pack.tex | 284 lines, 10 sections | **743 lines**, 18 sections, INV snapshot, R5 ledger, witness cross-ref, self-critique |
> | Q&A file | `qa.tex` (250 lines) | `anticipated-questions.tex` (348 lines, 30 explicitly numbered Q&A) |
> | Slides | `slides.tex` (361 lines) | `slides/main.tex` (423 lines, sub-directory layout) |
> | Witness | `crates/trios-phd/src/bin/defense_gate.rs` | **separate crate** `tools-defense-gate` + thin `assertions/witness/defense_gate.sh` call-through |
> | Rehearsal log | rubric only | rubric + 3-row scheduling-intent table |

## Directory layout

```
defense/
├── README.md                    — this file (race-condition disclosure)
├── slides/
│   ├── _outline.md              — auditor seed (preserved from 60d87cf)
│   └── main.tex                 — Beamer 30 frames (PR head)
├── examiner-pack.tex            — 50-pp external examiner summary (~743 lines)
├── rehearsal-log.md             — log of ≥3 rehearsals (auditor stamp + 3 slots)
├── anticipated-questions.tex    — 30 Q&A pairs, A 200-400 words
└── public-summary.md            — 1-page popular summary (CC-BY-4.0)
```

## Lane ownership

- **Skeleton** — auditor seed at `60d87cf` (preserved).
- **Body fill (alt PR, this branch)** — `phd-monograph-auditor` v1.0,
  agent `perplexity-computer-ld-defense`.
- **Body fill (other PR)** — agent `perplexity-computer-l12-hygiene` on
  the canonical branch `feat/phd-defense-skeleton`.

## R-rule trace

- **R1** Pure Rust witness in a dedicated crate `tools-defense-gate`;
  the `defense_gate.sh` exists only as a thin call-through (≤ 7 lines, no
  business logic) to satisfy the ONE SHOT v2.0 spec letter.
- **R5** No silent flips between Admitted and Proven; R5 honest ledger in
  examiner-pack.tex \S\ref{sec:examiner-admitted}.
- **R6** All edits stay inside `docs/phd/defense/` plus
  `crates/tools-defense-gate/` plus `assertions/witness/` plus the
  workspace `Cargo.toml` registration. No chapter `.tex` edits.
- **R7** Five gates in `tools-defense-gate`; every gate is a falsifier.
- **R8** Examiner pack ≤ 50 pp typeset; slides exactly 30; anticipated
  questions exactly 30 pairs.
- **R10** Five atomic commits, each scoped to one logical unit.
- **R11** All citations refer to entries already in `bibliography.bib`.
- **R12** Lee/GVSU style numeric `[n]` citation in examiner-pack.
- **R14** `\citetheorem{INV-N}` references appendix F (resolves
  post-#288 merge).

## Run the witness

```bash
cargo run -p tools-defense-gate
# or, via the thin call-through:
bash assertions/witness/defense_gate.sh
```

Expected output: five gates, all PASS, exit zero. Failure modes are
documented inline in `crates/tools-defense-gate/src/main.rs`.
