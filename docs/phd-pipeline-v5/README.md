# PhD Pipeline v5 — hero-fullwidth render from `ssot.chapters`

R5-honest pipeline that renders the 44-chapter TRINITY S³AI / GoldenSunflowers
PhD monograph from the Neon source-of-truth (`ssot.chapters.body_md`) into
per-chapter PDFs and one concatenated `monograph.pdf`, with the chapter hero
image:

- always positioned as the **first** content block,
- always rendered at **`width=\textwidth` = 100%** of the text block,
- always fixed in place with `[H]` (no floating),
- with `keepaspectratio` preserved.

## Why v5 fixes what v4 broke

| Issue in v4 | Fix in v5 |
|---|---|
| Hero floated `[htbp]` | Lua filter emits raw `[H]` figure |
| Width unset → ~60% | Hardcoded `width=\textwidth,keepaspectratio` |
| pandoc 3.x emits `Figure` block, not `Para+Image` — old filter never matched | New filter walks `Figure`/`Para`/`Plain` nodes |
| Caption duplicated (alt-text + italic body line) | Hero figure renders no caption; the body's italic `*Figure — ChNN: …*` line becomes the single caption |
| `φ` rendered as ` ` (replacement) inside italic spans | `\setmainfont{DejaVu Serif}` — italic variant has Greek glyphs |
| `$\sim$ 0.1` mis-paired by pandoc adjacent to other `$…$` | sed pre-pass replaces `$\macro$` fragments with their Unicode chars |
| Missing pandoc-emitted environments (`longtable`, `Shaded`, `Highlighting`) | Preamble loads `longtable`, declares `Shaded`/`Highlighting` envs and stubs all `\…Tok{}` highlighter macros as identity |

## Layout

```
phd_pipeline/
├── README.md                          ← this file
├── render.sh                          ← compile every src/*.md to build/<base>.pdf
├── compile_all.sh                     ← (optional) pull from Neon via psql then call render.sh
├── templates/chapter.template.tex     ← pandoc LaTeX template for one chapter
├── filters/force-fullwidth-hero.lua   ← move first image to top + raw-LaTeX [H] figure
├── src/                               ← Ch.1.md … Ch.34.md, App.A.md … App.J.md
├── assets/illustrations/              ← 44 hero PNGs, prefetched
└── build/                             ← per-chapter PDFs + monograph.pdf
```

## Local run

Prereqs: `pandoc` ≥ 3.1, `tectonic` ≥ 0.16, `qpdf` (or `pdftk`).

```bash
# 1) Pull body_md + illustrations from Neon (or copy src/ from elsewhere):
NEON_URL='postgres://…'  ./compile_all.sh

# 2) (alternative) compile from the included src/ already populated:
./render.sh

# 3) Concatenate into monograph.pdf:
qpdf --empty --pages \
  build/Ch.{1..34}.pdf build/App.{A..J}.pdf \
  -- build/monograph.pdf
```

## Output verified

- 44/44 chapters compile cleanly.
- Concatenated `monograph.pdf` is **196 pages, ≈22 MB**.
- Visual spot-check: Ch.1 (English-only), Ch.25 (Greek `φ` in title and math).

## Source-of-truth contract

`ssot.chapters.body_md` is the authoritative source. Each row's `body_md`
must begin with:

```markdown
![<alt text — same as title>](<https://…ch{NN}-{slug}.png>)

*Figure — Ch.NN: <title> (scientific triptych, 1200×800).*

# Ch.NN — <Title>

…
```

The lua filter assumes this layout. Migrating chapter content means
updating `ssot.chapters.body_md` only — there is no shadow markdown
checked into git.

## Anchor

φ² + φ⁻² = 3 — TRINITY · NEVER STOP.
