# Vasilev-Pellis Constants — Trinity S³AI DNA — v22.1 — QA report

**Artifact:** `/home/user/workspace/vasilev-pellis-constants-trinity-s3ai-dna-full-unified-v22_1.pdf`
**Contact sheet:** `/home/user/workspace/vasilev-pellis-v22_1-first12-contact.png`
**Size:** 41 963 356 B (40.0 MiB)
**SHA-256:** `98c305ed926dd667026ef6bca752daf1dd90137d366b463a55865949aa67c69d`
**Pages:** 122
**Build date:** 2026-05-15
**Source bases:** the new front matter (pages 1-2) is rendered from scratch by `docs/articles/_runner/src/build_v22_1_frontmatter.py`; pages 3-122 are the v22 atlas pages 4-123 (which were already content-stream-rewritten from the v21 atlas base by `rewrite_full_atlas.py`).

---

## Headline

The user rejected v22 because pages 1-3 were sparse/plain academic layout that did NOT match the atlas style of pages 4+. **v22.1 fixes this by rebuilding pages 1-2 from scratch in matching atlas style** and dropping the sparse v22 pages 1-3 entirely. The first 12 pages now visually unify (see contact sheet).

- **Pages: 122** (full article length).
- **qpdf --check:** PASS, **linearized**.
- **Annotations: 0** (no highlight/comment/text-markup/popup — links-only policy met).
- **Image streams: 52** (51 from v21 atlas + 1 cover-plate triptych re-embedded on new page 1).
- **All 6 forbidden visible strings → 0** in `pdftotext` output.
- **All required brand/Catalog42 strings present**.

## Visual unification — first 12 pages

Contact sheet at `/home/user/workspace/vasilev-pellis-v22_1-first12-contact.png` (2.2 MiB, 4×3 grid).

| Page | Content | Style verdict |
|------|---------|---------------|
| 1 (NEW) | Atlas cover: title `Vasilev-Pellis Constants`, subtitle `A Three-Strand TRI-1 DNA Architecture under the Trinity S³AI DNA brand`, authors, anchor identity `φ² + φ⁻² = 3`, cover-plate triptych (Input Constants / Symbolic Search / Validation), caption, dense intro paragraph, Catalog42 wording lock heading + bullets | **Atlas-styled** |
| 2 (NEW) | Atlas-styled title heading (Low-Complexity Algebraic Representations…), authors, version line, Abstract heading, dense abstract block, Seed/Vesica/Sprout triptych, caption | **Atlas-styled** |
| 3   | Paper-1 §1 Introduction with Seed/Vesica/Sprout triptych (was v22 page 4) | Atlas-styled |
| 4   | §1.3 What This Paper Does Not Claim, §1.4 Primary Objectives | Atlas-styled |
| 5   | §1.5 Paper Organization | Atlas-styled |
| 6   | §1.6 Notation, §2 Background and Related Work | Atlas-styled |
| 7   | §2 Background-and-Related-Work triptych + body | Atlas-styled |
| 8   | §2.3 Multiple Testing Corrections | Atlas-styled |
| 9   | §3 Symbolic Hypothesis Class, §3.1 Symbolic Regression, §3.2 Combinatorial Complexity | Atlas-styled |
| 10  | §3.3 Properties of HC, §3.4 Complexity Levels, §3.5 Continuous Limits | Atlas-styled |
| 11  | §3 Logarithmic Embedding triptych + §3.6 Logarithmic Embedding Geometry | Atlas-styled |
| 12  | §3.7 Continuous Embedding theorem | Atlas-styled |

**Visual verdict: unified.** All 12 pages share the cream background `#F7F6F2`, dark body text `#28251D`, header `Vasilev-Pellis Constants` / `Trinity S³AI DNA` at top, footer `— N —` page numbers, dense 10.5pt DejaVuSerif body, 12-pt bold section headings, triptych plates at consistent body width. No alien sparse pages, no plain academic layout intrusions, no different-style errata sheets.

## Brand and header lock — verified

| Field | Required | Verified |
|-------|----------|----------|
| Visible title (cover, every running-header left) | `Vasilev-Pellis Constants` | 115 occurrences |
| Visible brand (every running-header right)       | `Trinity S³AI DNA` (real U+00B3 superscript) | 115 occurrences |
| Authors line (cover + abstract page) | `Dmitrii Vasilev · Stergios Pellis · Scott Olsen` | both pages |
| Anchor identity (cover) | `φ² + φ⁻² = 3` | rendered correctly with proper Greek + superscript glyphs |
| Scott Olsen Tier-D section (§12.5) | dedicated section, golden balance motif | present, 9 occurrences |

## Forbidden strings — all zero in rendered text

```
$ pdftotext vasilev-pellis-constants-trinity-s3ai-dna-full-unified-v22_1.pdf - | grep -c -F '<string>'
```

| Forbidden string                                          | Count |
|-----------------------------------------------------------|------:|
| `PhD-style Research Article`                              | **0** |
| `Pellis–Trinity Constants — full article`                 | **0** |
| `Pellis–Trinity Constants` (en-dash)                      | **0** |
| `Pellis-Trinity Constants` (as title/brand)               | **0** |
| `42/42`                                                    | **0** |
| `[link]`                                                  | **0** |

## Required strings — all present

| Required string                            | Count |
|--------------------------------------------|------:|
| `Vasilev-Pellis Constants`                 |   115 |
| `Trinity S³AI DNA`                         |   115 |
| `Scott Olsen`                              |     9 |
| `golden balance`                           |     2 |
| `42 declared`                              |     3 |
| `19 verified`                              |     4 |
| `23 UnderRevision`                         |     5 |
| `Catalog42`                                |    37 |
| `min(1, 15) = 1`                           |     2 |

## Annotation audit

```
total annotations: 0
```

No highlights, no comments, no popups, no text-markup. Link-annotation count is 0 (v21 base also had 0). The /Annots policy is met.

## Image audit

```
$ pdfimages -list vasilev-pellis-constants-trinity-s3ai-dna-full-unified-v22_1.pdf | tail -n+3 | wc -l
52
```

52 image streams = 51 preserved from v21 atlas pages 4-123 (cover-plate triptych on old p1, Seed/Vesica/Sprout, A5 Flavor Symmetry, Falsification Ledger, etc.) + 1 new embed of the cover-plate triptych on v22.1 page 1.

## qpdf / pdfinfo — verbatim

```
$ qpdf --check vasilev-pellis-constants-trinity-s3ai-dna-full-unified-v22_1.pdf
PDF Version: 1.7
File is not encrypted
File is linearized
No syntax or stream encoding errors found; the file may still contain
errors that qpdf cannot detect

$ pdfinfo vasilev-pellis-constants-trinity-s3ai-dna-full-unified-v22_1.pdf
Title:           Vasilev-Pellis Constants (Trinity S³AI DNA, v22.1 unified-frontmatter full atlas)
Author:          gHashTag/trios
Subject:         Full PhD-style atlas article with unified atlas-styled front matter from page 1
Keywords:        Vasilev-Pellis, Trinity S3AI DNA, Catalog42, golden-balance, Olsen, Tier-D
Creator:         tri article (repo runner) — v22.1 atlas-unified front-matter rebuild
Producer:        pymupdf direct page render + content-stream rewrite of v21 atlas
Pages:           122
PDF version:     1.7
```

## What changed vs v22

| Aspect | v22 (rejected) | v22.1 (this build) |
|---|---|---|
| Page count | 123 | 122 |
| Page 1 style | Sparse cover (giant blank upper half, isolated triptych at bottom) | **Atlas-styled dense cover** with title, subtitle, authors, anchor identity, cover-plate triptych at body density, caption, intro paragraph, Catalog42 wording lock |
| Page 2 style | Sparse 7-line abstract with 80% blank | **Atlas-styled abstract page** with title heading, authors, version, Abstract heading, dense abstract paragraphs, Seed/Vesica/Sprout triptych, caption |
| Page 3 style | Plain academic paper-1 title page (no figure) | Removed; replaced by atlas Paper-1 §1 Introduction (was v22 p4) |
| Front-matter unification | NO — break visible at p1/p2/p3 vs p4+ | **YES — verified by contact sheet of first 12 pages** |
| Forbidden strings | 0 | 0 |
| Brand strings | 113× / 113× | 115× / 115× |
| Annotations | 0 | 0 |
| Image streams | 51 | 52 |

## Reproduction

From repo root, on branch `docs/pellis-trinity-olsen-quote-block`:

```bash
# Step 1 (already done in PR #824): rewrite v21 base content stream
python3 docs/articles/_runner/src/rewrite_full_atlas.py \
  --in  /home/user/workspace/tri-article-pellis-trinity-phdstyle-atlas-v21-full-no-annots.pdf \
  --out /tmp/v22.pdf
qpdf --linearize /tmp/v22.pdf \
  /home/user/workspace/vasilev-pellis-constants-trinity-s3ai-dna-full-unified-v22.pdf

# Step 2 (this build): rebuild atlas-styled front matter and splice
python3 docs/articles/_runner/src/build_v22_1_frontmatter.py \
  --in  /home/user/workspace/vasilev-pellis-constants-trinity-s3ai-dna-full-unified-v22.pdf \
  --out /tmp/v22_1.pdf \
  --cover /tmp/cover_triptych.png \
  --plate2 /tmp/seed_vesica_sprout.png
qpdf --linearize /tmp/v22_1.pdf \
  /home/user/workspace/vasilev-pellis-constants-trinity-s3ai-dna-full-unified-v22_1.pdf
```

## Caveats (honest)

- The new pages 1-2 are *generated* by pymupdf using DejaVu fonts via `insert_textbox(fontfile=...)`. They match the atlas style closely but are not pixel-identical to the v21 atlas pages (those use slightly different DejaVu widths inside their original ReportLab origin). The visual consistency check is by-eye on the contact sheet, not by glyph metrics.
- The `ℓ` (U+2113) glyph is not in DejaVu Serif. The abstract reads `ell^1-bounded exponent lattice` rather than `ℓ¹-bounded`. This avoids the `□` fallback that pymupdf produces for missing glyphs.
- Page numbering restarts at `— 1 —` on the new front matter, then the body retains its original v21 numbering. This is a single one-page offset (new-cover is "page 1" but v21's old §1 Introduction also had `— 1 —` after the cover and abstract that were removed). The reader sees consistent footer style; logical numbering is consistent within the v21 body section.
- No PDF was built by the Rust `tri article` service because that subcommand is still unimplemented (same blocker as #821, #822, #824 baseline). v22.1 uses pymupdf direct rendering for the new front matter, which is the most truthful approach short of a real source rebuild.
