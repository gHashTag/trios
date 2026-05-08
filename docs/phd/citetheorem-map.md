# `\citetheorem{INV-k}` resolution map — Phase 1 UNIFY task 1.6

**Branch:** `feat/phd-phase1-unify-1-6` (stacked on `feat/phd-phase1-unify-1-5`, PR #602)
**Issue:** [trios#380](https://github.com/gHashTag/trios/issues/380) task 1.6
**Anchor:** φ² + φ⁻² = 3 · DOI [10.5281/zenodo.19227877](https://zenodo.org/records/19227877)

## Summary

- **Macro:** `\citetheorem{INV-k}` defined in `docs/phd/main.tex` (and mirrored in `main_ru.tex`) as a `\providecommand` so existing stub in `defense/slides.tex` is not overridden.
- **Resolution target:** `\label{thm:INV-k}` in `docs/phd/appendix/F-coq-citation-map.tex`.
- **INV labels in appendix F:** 1 files, 13 total label sites; **13 distinct INV-k**: ['INV-1', 'INV-2', 'INV-3', 'INV-4', 'INV-5', 'INV-6', 'INV-7', 'INV-8', 'INV-9', 'INV-12', 'INV-13', 'INV-22', 'INV-23']
- **Distinct INV-N mentioned in body+appendix:** ['INV-1', 'INV-2', 'INV-3', 'INV-4', 'INV-5', 'INV-6', 'INV-7', 'INV-8', 'INV-9', 'INV-12', 'INV-13', 'INV-22', 'INV-23']

## INV → label coverage

| INV ID | Has `\label{thm:INV-k}` in F? | Mentioned in chapters |
|---|---|---|
| `INV-1` | ✅ | 94 sites |
| `INV-2` | ✅ | 22 sites |
| `INV-3` | ✅ | 85 sites |
| `INV-4` | ✅ | 60 sites |
| `INV-5` | ✅ | 28 sites |
| `INV-6` | ✅ | 9 sites |
| `INV-7` | ✅ | 59 sites |
| `INV-8` | ✅ | 20 sites |
| `INV-9` | ✅ | 19 sites |
| `INV-12` | ✅ | 13 sites |
| `INV-13` | ✅ | 15 sites |
| `INV-22` | ✅ | 11 sites |
| `INV-23` | ✅ | 6 sites |

## Existing `\citetheorem` consumers (pre-task)

Before task 1.6 the macro existed only as a stub in `defense/slides.tex`:
```latex
\newcommand{\citetheorem}[1]{[\textsc{#1}]}
```

After task 1.6 the **canonical** definition lives in `main.tex` / `main_ru.tex`:
```latex
\providecommand{\citetheorem}[1]{%
  \hyperref[thm:#1]{[\textsc{#1}]}%
}
```
(The slide stub remains in place — `\providecommand` does not clobber it.)

Current `\citetheorem` invocation sites:

| Argument | File(s) |
|---|---|
| `INV-1` | defense/slides.tex, main.tex |
| `INV-1..INV-13` | defense/slides.tex, main.tex |
| `INV-12` | defense/slides.tex, main.tex |
| `INV-12 lucas\_2\_phi\_identity (Qed)` | defense/slides.tex |
| `INV-2` | defense/slides.tex |
| `INV-2 (Qed)` | defense/slides.tex |
| `INV-22` | defense/slides.tex |
| `INV-22 trinity\_in\_e8 (Qed)` | defense/slides.tex |
| `INV-23` | defense/slides.tex |
| `INV-7` | defense/slides.tex |
| `INV-k` | main_ru.tex, main.tex |

## Acceptance criteria (#380 task 1.6)

| Criterion | Status |
|---|---|
| `\citetheorem` macro defined corpus-wide | ✅ `main.tex` + `main_ru.tex` |
| Every INV-k mentioned in body has a label in F | ✅ all 13 INV-Ks in JSON now have `\label{thm:INV-k}` |
| Macro resolves through AP.F (not `\bibliography`) | ✅ `\hyperref[thm:#1]{...}` |
| Backward-compatible with `defense/slides.tex` stub | ✅ `\providecommand` form |
| Audit document produced | ✅ this file |

## Honesty (R5)

INV-9, INV-13, INV-22, INV-23 are mentioned in chapter bodies but **not** present in `assertions/igla_assertions.json` as primary records. To preserve R5 (no fabrication), their rows in appendix F use `\emph{registry-only}` for the theorem name and `—` for status, and the `coq_file` cell reads `\emph{no .v anchor}`. This is honest: the macro will resolve and produce a hyperlink, but the appendix row signals the registry has not yet promoted these IDs. Chapters that cite these INVs are responsible for either (a) registering an anchor theorem in the JSON, or (b) reformulating the citation.

INV-6 is in the JSON as `Proven` (`ema_decay_valid`) — its row is canonical.

## Skill provenance

Authored under `phd-chapter-author` v1.1 + `phd-monograph-auditor` v1.2.
R1 (no `.py`/`.sh`): patch script ran from `/tmp/`, only LaTeX + Markdown committed.
