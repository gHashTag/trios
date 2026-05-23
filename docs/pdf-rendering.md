# TRIOS PhD PDF Rendering — Canonical Rules

This document is the source of truth for how the TRIOS PhD monograph is
rendered to PDF. Every agent and developer touching the renderer, the
templates, or the chapter sources MUST follow these rules.

It is repo-canonical and overrides ad-hoc renderer scripts.

---

## TRIOS_PHD_CANONICAL_PIPELINE

The only supported PDF pipeline is:

```
Rust TRIOS MCP / trios-phd
        │
        ▼
Railway / Postgres (ssot.chapters)   ← single source of truth (SSOT)
        │
        ▼
Markdown (body_md + leading hero image)
        │
        ▼   pandoc + Lua filter (filters/force-fullwidth-hero.lua)
        │   + template (templates/chapter.template.tex
        │     or docs/phd/main.tex for the full book)
        ▼
LaTeX (.tex)
        │
        ▼   tectonic
        ▼
PDF (print/export target — NOT a source of truth)
```

Reference entry points:

- Per-chapter compile: `crates/trios-phd` → `trios-phd compile-chapters`
  (`crates/trios-phd/src/main.rs`, around the `CompileChapters` subcommand).
- Full book build: `docs/phd/main.tex` (the `book` class scaffold),
  compiled by `tectonic` via `trios-phd build-book` /
  `trios-phd compile-resilient`.
- Per-chapter pandoc template: `templates/chapter.template.tex`.
- Hero-image Lua filter: `filters/force-fullwidth-hero.lua`.

**Do not invent a parallel ReportLab / Python PDF generator.** PDF is an
export from the renderer; it is not a hand-authored artefact.

---

## TRIOS_PHD_RENDERER_FIRST

If a placement, typography, or image rule must change, change it in the
renderer (template, Lua filter, or `docs/phd/main.tex`), NOT by editing
the per-chapter `.tex` files or by post-processing PDFs.

Order of preference for any structural change:

1. `templates/chapter.template.tex` (per-chapter pandoc template).
2. `docs/phd/main.tex` (book preamble for the full monograph).
3. `filters/force-fullwidth-hero.lua` (Markdown → AST transform).
4. As a last resort: a Rust function in `crates/trios-phd`.

Editing individual chapters is allowed only for content. Visual /
placement / image-train concerns belong to the renderer layer.

---

## TRIOS_PHD_STYLE_LOCK

The accepted PhD visual style is locked:

- White academic title page.
- Serif typography (`DejaVu Serif` for main, math via classic CM).
- Large black-and-white engraved / ornamental TRIOS S3AI hero panels.
- Book-style margins (`docs/phd/main.tex` geometry block).
- Large, centered hero images — never thumbnails.

QA baseline (manual build that defines "accepted"):

- 150 A4 pages.
- `qpdf --check`: clean.
- Exact duplicate long paragraphs: 0.
- Duplicate numbered headings: 0.
- Cyrillic hits in the body: 0 (English-only repo docs).
- Secret / stale / math anomaly hits: 0.
- Very short non-empty pages: 0.
- Image-heavy / low-context candidate pages: at most 1 (the title page).

Any renderer change that regresses this baseline is a failure.

---

## TRIOS_PHD_NO_IMAGE_TRAIN

This is the non-negotiable rule that motivates the current renderer
configuration.

### Statement

Hero panels MUST NOT be laid out as a gallery or image train. Each hero
panel must be semantically anchored to the nearest substantive heading
and its body text. A heading must never be orphaned at the bottom of a
page above its hero figure, and hero figures from neighbouring sections
must never collapse into a multi-image train on a single page.

### How it is enforced

A **soft keep-together** vertical reservation, sized to fit a heading
plus a full hero figure (~`0.58\textheight`), is inserted before every
`\section` and `\chapter` via the `needspace` package:

```latex
\usepackage{needspace}

\makeatletter
\let\trios@orig@section\section
\renewcommand{\section}{%
  \Needspace*{0.58\textheight}%
  \trios@orig@section
}
\let\trios@orig@chapter\chapter
\renewcommand{\chapter}{%
  \Needspace*{0.58\textheight}%
  \trios@orig@chapter
}
\makeatother
```

This block lives in:

- `docs/phd/main.tex` — for the full monograph build.
- `templates/chapter.template.tex` — for the per-chapter pandoc build.
- `\chapterhero{}{}` in `templates/chapter.template.tex` is preceded by
  `\Needspace*{0.58\textheight}` as well, so the chapter heading + hero
  cannot be split.

`\Needspace*` (starred form) is critical: it triggers a page break only
when the remaining vertical space is insufficient AND we are not already
at the top of a page. It does not insert gratuitous breaks.

### Why hard `\clearpage` is FORBIDDEN

`\clearpage` before every section was tried and rejected. It produced
short title-only pages (a heading at the top of a fresh page, then a
hero figure on the next page) and broke the QA baseline. **Never insert
`\clearpage` before sections or hero figures as a workaround.** If a
heading still appears orphaned, raise the reservation size (e.g. to
`0.62\textheight`) — do not switch to `\clearpage`.

### What about the Lua filter?

`filters/force-fullwidth-hero.lua` promotes exactly one hero image per
chapter to full text width at block position 1. Additional standalone
images stay where the author placed them, anchored to their nearest
heading. The filter is part of the no-image-train discipline, not in
conflict with it.

### Image size

Hero images remain large, centered, full text width, and PhD-style. The
no-image-train rule is about placement, not about shrinking or removing
images.

---

## QA commands

Run these after any renderer-touching change:

```bash
# Build the per-chapter PDFs (no secrets needed beyond local pandoc + tectonic).
cargo run -p trios-phd -- compile-chapters \
    --chapters-dir docs/golden-sunflowers \
    --template     templates/chapter.template.tex \
    --lua-filter   filters/force-fullwidth-hero.lua \
    --out-dir      docs/golden-sunflowers/pdf

# Build the full monograph (requires the full chapter source tree under docs/phd/).
cargo run -p trios-phd -- build-book

# Validate the produced PDF.
qpdf --check                  build/phd.pdf
pdfinfo                       build/phd.pdf
pdftotext  -layout            build/phd.pdf build/phd.txt

# QA scans on the extracted text.
grep -nE '[А-Яа-яЁё]'            build/phd.txt  || echo "OK: no Cyrillic"
grep -nE 'TODO|FIXME|XXX|STALE'  build/phd.txt  || echo "OK: no stale markers"
grep -nE '(password|secret|token|api[_-]?key)=' build/phd.txt \
                                                || echo "OK: no secrets"

# Duplicate-heading and duplicate-paragraph scans live in trios-phd's
# audit subcommand and the tools under tools/page_gate/ — run them and
# compare to the baseline:
cargo run -p trios-phd -- audit
```

Visually inspect:

- The title page (white academic, large engraved S3AI panel).
- The first body pages of each Part — confirm no section heading sits
  alone at the bottom of a page above its hero figure.
- Any chapter that previously triggered an image train — confirm only
  one hero per chapter at the top, and any in-body images sit with
  their semantic owners.

---

## What NOT to do

- Do NOT add `\clearpage` before sections or hero figures.
- Do NOT shrink hero images to "fix" pagination — adjust
  `\Needspace*{...}` instead.
- Do NOT post-process the rendered PDF to move images.
- Do NOT introduce a parallel Python / ReportLab generator.
- Do NOT edit individual chapter `.tex` files for placement concerns.
- Do NOT print, log, or commit Railway tokens / database passwords
  while running the pipeline.

---

## Pointers

- `crates/trios-phd/src/main.rs` — Rust orchestration of the pipeline.
- `docs/phd/main.tex` — book preamble, includes `needspace` and the
  `\section` / `\chapter` keep-together wrapper.
- `templates/chapter.template.tex` — per-chapter pandoc template,
  includes the same wrapper and `\chapterhero`.
- `filters/force-fullwidth-hero.lua` — one-hero-per-chapter promotion.
- `assets/illustrations/` — hero panel sources.
- `AGENTS.md` — short pointer to this document under TRIOS_PHD.
