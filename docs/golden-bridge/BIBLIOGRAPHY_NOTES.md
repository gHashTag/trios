# Bibliography for Paper 1 — what was found, what was built

## What was there

Paper 1 had **no bibliography**. Not a thin one — none:

- zero `\cite{}` commands anywhere in `chapters/p1-*.tex`
- no `.bib` file, no `\bibliography`, no bibliography package in `main.tex`
- instead, 18 `\subsection{References}` blocks, one per chapter

Those 18 blocks were not 18 different lists. They were **the same three lines
copied verbatim into every chapter**: the Zenodo monograph, the Pellis viXra
preprint, and the Sherbon HAL preprint. Three preprints, zero peer-reviewed
sources, repeated 18 times.

The same block appears in **38 files across the whole compendium**, so Papers 2
and 3 carry the identical hole — 20 files still do, see *Remaining work*.

Meanwhile the prose cited real literature properly — Rissanen, Barron/Rissanen/Yu,
Grünwald, Chaitin, PDG, CODATA, Planck, Udrescu & Tegmark, Angelis et al. — but
only as inline text. None of it was in any list.

Appendix D held a source list where **all nine URLs were empty**
(`- PDG 2024 Physical Constants table: URL:` with nothing after). That template
was never filled in. It has been replaced by real citations.

## What was built

`references.bib` — 14 entries. Every one verified, none typed from memory:

| Source | How verified |
|---|---|
| Rissanen 1978, Barron–Rissanen–Yu 1998, Benjamini–Hochberg 1995, PDG 2024 | Crossref, plus cross-check against the already-audited bibliography of the Pellis–Vasilev–Olsen short paper |
| Angelis et al. 2023, Udrescu & Tegmark 2020, CODATA 2022, Planck 2018 VI | Crossref API, by DOI or exact-title query |
| Grünwald 2007, Chaitin 1987, Olsen 2006 | books; publisher, year, ISBN |
| Pellis viXra, Sherbon HAL, Vasilev Zenodo | marked **preprint / self-published** in a `note` field |

Two Crossref queries initially returned **wrong** matches — CODATA *2014* and
Planck *2015* — from fuzzy bibliographic search. Both were re-queried by exact
title and DOI. Do not trust a single fuzzy Crossref hit for these.

The three non-peer-reviewed sources carry an explicit note. They are the claims
*under test*, not supporting evidence, and the file says so. Do not silently
upgrade them to journal status.

## Changes to the chapters

- 18 duplicated `\subsection{References}` blocks removed.
- **34 citations** inserted into the prose (28 in pass 1, 6 in pass 2), via
  `cite_p1.pl` and `cite_p1_pass2.pl` — kept in the tree so the conversion is
  reproducible and reviewable rather than a mystery diff.
- Appendix D's hollow URL list replaced with real citations.
- `main.tex`: added `\usepackage[numbers,sort&compress]{natbib}`,
  `\bibliographystyle{plainnat}`, `\bibliography{references}`.

## Verification

All 9 citation keys used in Paper 1 resolve against `references.bib`
(set difference used-minus-defined is empty). A minimal document citing all 14
entries compiles with **0 BibTeX warnings** and renders every entry with its
DOI.

## The compendium does not build — and did not before this change

`tectonic main.tex` fails at `frontmatter/fm-02-attribution.tex:67`:

```
- \textbf{Pellis fine-structure formula (golden-angle)}: ... + (3$\varphi$$)^{-5}
```

The `$$` opens display math mid-line. That file was never touched by this work.

A second, independent failure sits at `chapters/p1-03-symbolic-grammar.tex:51`:
`R^+` uses `^` in text mode. Also outside the edited region.

Both are pre-existing. Together with the 436 unfilled `\_\_C0\_\_`-style
placeholders across 17 files, they mean **this document has never been compiled
successfully**. The bibliography is correct and verified in isolation; it cannot
be verified in situ until the math errors are fixed.

## Remaining work

1. **Fix the LaTeX math errors** so the compendium compiles at all. Unknown
   count — two found in the first two build attempts, so expect more.
2. **20 files in Papers 2 and 3** still carry the three-preprint boilerplate.
   `references.bib` already covers the shared sources; the same two scripts
   generalise.
3. **436 placeholders** (`\_\_C0\_\_`, `\_\_M0\_\_`, …) across 17 files,
   including the CRediT table in `frontmatter/fm-02-attribution.tex`.
4. Decide **Catalog42 vs Catalog15** — the compendium says Catalog42
   throughout; the short paper says Catalog15.
