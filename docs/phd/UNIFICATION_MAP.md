# Flos Aureus Unification Map (v1)

**Branch:** `feat/phd-unify-flos-aureus`
**Date:** 2026-05-12
**Anchor:** `phi^2 + phi^-2 = 3`
**Defense:** 2026-06-15 (T-33 days)
**Primary language:** English (`docs/phd/main.tex`)
**Secondary:** Russian translation (`docs/phd/main_ru.tex` — kept alongside EN)

## Premise

Per operator directive *"слить логично в рамках идеи Flos Aureus"*, the two
parallel chapter strands are unified into a single monograph titled
**Flos Aureus — The Golden Flower**:

- Book I — **Flos Aureus** (was `fa_*`)
- Book II — **Trinity S³AI** (was `ch_00..ch_34`)
- Book III — **Silicon Strand** (was `ch_35_mesh_node`)

All 70 chapters are renumbered as a single sequential strand
`flos_00..flos_69`.

## Filename Map (70 chapters)

| Old | New | Book | New # |
|-----|-----|------|-------|
| `chapters/fa_00.tex` | `chapters/flos_00.tex` | I | 0 |
| `chapters/fa_01.tex` | `chapters/flos_01.tex` | I | 1 |
| … (fa_02..fa_32) | (flos_02..flos_32) | I | 2..32 |
| `chapters/fa_33.tex` | `chapters/flos_33.tex` | I | 33 |
| `chapters/ch_00.tex` | `chapters/flos_34.tex` | II | 34 |
| `chapters/ch_01.tex` | `chapters/flos_35.tex` | II | 35 |
| … (ch_02..ch_33) | (flos_36..flos_67) | II | 36..67 |
| `chapters/ch_34.tex` | `chapters/flos_68.tex` | II | 68 |
| `chapters/ch_35_mesh_node.tex` | `chapters/flos_69.tex` | III | 69 |

## Untouched

- `frontmatter/*.tex` (11 files)
- `appendix/[A-N]-*.tex` (14 files)

## Labels & Cross-refs

- Existing `\label{ch:…}` keys are **semantic** (e.g. `ch:golden-seed`,
  `ch:igla-architecture`) — they DO NOT encode `fa_NN` / `ch_NN`.
- Verified: zero `\ref{ch:fa-NN}` / `\cref{ch:fa-NN}` / `\ref{ch:ch-NN}` /
  `\cref{ch:ch-NN}` in the codebase.
- **No label sweep required.** Rename is purely file-path-level.

## Affected files (this PR)

1. `docs/phd/main.tex` — 70 `\include{}` paths updated (`fa_NN`/`ch_NN`
   → `flos_NN`)
2. `docs/phd/main_ru.tex` — 70 `\include{}` paths updated
3. `docs/phd/cross-ref-audit.md` — 2 354 slug replacements
4. `docs/phd/chapter-headers-audit.md` — 118 slug replacements
5. `docs/phd/rag/rag_chunks.jsonl` — 1 478 slug replacements

## SSOT migration (R6, out-of-band)

`ssot.embeddings.chapter_slug` migrated via `psycopg2` admin in single
transaction:

- 739 chapter rows updated (`fa_NN` → `flos_NN`; `ch_NN` →
  `flos_(NN+34)`; `ch_35_mesh_node` → `flos_69`)
- 0 leftover `fa_/ch_` slugs
- 70 distinct `flos_*` slugs present
- Anchor `phi^2 + phi^-2 = 3` preserved in 100 % of 1 063 chunks

## R-Rule preservation

| Rule | How preserved |
|------|---------------|
| R1 CROWN (Rust-only pipeline) | Renames are `git mv` + Python sed (not pipeline); Postgres update is admin one-shot (psycopg2), not pipeline |
| R5 HONEST | `\admittedbox{}` blocks moved verbatim with each file |
| R6 SSOT | Postgres `ssot.embeddings` migrated atomically |
| R7 ANCHOR | `phi^2 + phi^-2 = 3` 100 % preserved |
| NEVER push to main | Work on `feat/phd-unify-flos-aureus`; merge via PR |

## Static validation

- `main.tex`: 93 `\include{}` paths, **0 missing**
- `main_ru.tex`: 84 `\include{}` paths, **0 missing**

## Decision: dual-build retained

EN (`main.tex`) is the primary citable artifact (arXiv / Zenodo /
NeurIPS). RU (`main_ru.tex`) is retained as parallel translation; both
build to PDF. Post-defense the RU build will move under
`docs/phd/translations/ru/` — out of scope for this PR.
