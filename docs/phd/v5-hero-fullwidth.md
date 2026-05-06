# PhD v5 — Hero illustration full-width pipeline

This document records the v5 change to the per-chapter Markdown → TeX → PDF
pipeline. The goal: every chapter's hero illustration is rendered as the
**first** block of the chapter and **always** spans the full text width.

## Diff vs v4

| Aspect             | v4 (previous)              | v5 (this change)                                  |
|--------------------|----------------------------|---------------------------------------------------|
| Hero position      | wherever in the body_md    | always block #1 (Lua filter promotion)            |
| Image width        | implicit (~60%)            | `\linewidth` via `width=100% .hero-fullwidth`     |
| Aspect ratio       | could break                | `keepaspectratio`                                 |
| Float behaviour    | `[htbp]` (drifts)          | `[H]` (fixed in place)                            |
| Caption            | Markdown alt-text only     | `\caption*{}` italic, no figure number            |
| body_md prefix     | varied                     | normalized by `migrations/005_hero_fullwidth.sql` |

## Files added in this PR

- `templates/chapter.template.tex` — Pandoc LaTeX template defining
  `\chapterhero{path}{caption}` and emitting it as the first block.
- `filters/force-fullwidth-hero.lua` — Pandoc Lua filter that promotes the
  first standalone Markdown image to position 1, forces `width=100%`,
  adds the `.hero-fullwidth` class, and exposes `hero-image` /
  `hero-caption` template vars.
- `scripts/compile_chapter.sh` — invokes pandoc + tectonic with the
  template and filter wired in.
- `migrations/005_hero_fullwidth.sql` — idempotent normalization of every
  `ssot.chapters.body_md` so it begins with the canonical hero line:
  ```
  ![<title>](<illustration_url>){width=100% .hero-fullwidth}
  ```
  and nulls `body_pdf_url` to trigger recompile.

## Operational notes

- Schema in Neon uses `body_md`, `body_pdf_url`, `illustration_url`,
  `illustration_path` (snake_case). There is no `slug` column.
- `ch_num` is **text** (`Ch.1`, `App.A`, `FA.07`, `FM.01`, `AP.A`),
  not an integer — never use `WHERE ch_num BETWEEN 1 AND 44`.
- Chapter inventory is **100 rows**: `Ch.0..Ch.34` (35) + `App.A..App.J`
  (10) + `AP.A..AP.H` (8) + `FA.00..FA.33` (34) + `FM.01..FM.11` (11) +
  `Ch.35..Ch.36` (2). 98 of these have an illustration and are touched by
  the migration.
- After migration:
  - 98 / 100 chapters begin with `![..](..){width=100% .hero-fullwidth}`
  - 100 / 100 have `body_pdf_url = NULL`, ready for a v5 compile pass.

## Compile trigger

```bash
psql "$NEON_URL" -f migrations/005_hero_fullwidth.sql
curl -X POST "https://t27.ai/api/compile/all?version=v5" \
  -H "Authorization: Bearer $ACC5_TOKEN"
curl "https://t27.ai/phd/goldensunflowers_v5.pdf" -o /tmp/phd_v5.pdf
```
