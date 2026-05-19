#!/usr/bin/env python3
"""build_v22_1_frontmatter.py — v22.1 atlas-styled front matter.

The v22 PDF (123 pages, built by rewrite_full_atlas.py) has its first
three pages in a plain-academic layout that does NOT match the atlas
visual language used from page 4 onward (dense body text, triptych
plate, atlas header/footer). This script:

  1. Renders TWO new front-matter pages in the matching atlas style:
       new-1: cover with Vasilev-Pellis Constants title, brand,
              authors, anchor identity, cover triptych image
              (re-uses the v21 cover-plate xref), and a dense
              body block (abstract + Catalog42 contract summary).
       new-2: extended abstract with the longer Paper-1 abstract,
              keywords, and an opener triptych image.
     Both pages use the same A4 595.28x841.89 size, the same body
     bbox (62.7-532.6 horizontal, 82.2-776.3 vertical), the same
     5pt Helvetica header band ("Vasilev-Pellis Constants" left /
     "Trinity S^3AI DNA" right), the same 10.5pt DejaVuSerif body,
     and the same "-- N --" page-number footer at y=803.5.
  2. Concatenates: new-1, new-2, and pages 4..123 of v22.pdf
     (skipping v22 pages 1, 2, 3 which are the sparse front matter).
     Resulting page count: 2 + (123 - 3) = 122.

Inputs:
  --in     v22 PDF (the 123-page output of rewrite_full_atlas.py)
  --out    v22.1 PDF (122 pages, unified atlas style from page 1)
  --cover  PNG: cover triptych image (xref 329 extracted)
  --plate2 PNG: secondary plate for the abstract page

Output is then linearized externally with qpdf.
"""
from __future__ import annotations

import argparse
import sys

import fitz

A4_W = 595.27559
A4_H = 841.8898

# Match atlas page conventions exactly.
HEADER_Y = 53.0          # baseline
HEADER_FONT_SIZE = 9.0   # original was 9pt italic; we use Helvetica 9pt for clarity
HEADER_COLOR = (0x22 / 255, 0x22 / 255, 0x22 / 255)
HEADER_LEFT = "Vasilev-Pellis Constants"
HEADER_RIGHT = "Trinity S³AI DNA"

BODY_X0 = 62.7
BODY_X1 = 532.6
BODY_TOP = 82.2
BODY_BOTTOM = 776.3

BODY_FONT = "DejaVuSerif"          # matches atlas body
BODY_FONT_BOLD = "DejaVuSerif-Bold"
BODY_FONT_ITAL = "DejaVuSerif-Italic"
BODY_SIZE = 10.5
BODY_LEADING = 14.0
BODY_COLOR = (0x28 / 255, 0x25 / 255, 0x1d / 255)

HEADING_SIZE = 12.0
TITLE_SIZE = 27.0   # cover title
SUBTITLE_SIZE = 12.0
AUTHORS_SIZE = 11.5

FOOTER_Y = 808.0
FOOTER_FONT_SIZE = 9.5

# Horizontal rule above footer
RULE_Y_TOP = 791.0
RULE_COLOR = (0x80 / 255, 0x80 / 255, 0x80 / 255)


def register_fonts(doc: fitz.Document):
    """Try to register DejaVu serif fonts. Fall back to Helvetica if unavailable."""
    candidates = [
        ("/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf", BODY_FONT),
        ("/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf", BODY_FONT_BOLD),
        ("/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf", BODY_FONT_ITAL),
    ]
    fontmap = {}
    import os
    for path, want in candidates:
        if os.path.exists(path):
            try:
                doc.tdoc = doc  # keep reference for some versions
                # pymupdf needs fontfile registered per page; we'll pass `fontfile=` later
                fontmap[want] = path
            except Exception:
                pass
    return fontmap


def draw_header_footer(page: fitz.Page, page_no: int, fontmap: dict):
    """Draw the running header (left + right) and the page-number footer."""
    # Header left/right at 9pt Helvetica (clear, contrast)
    page.insert_text(
        (BODY_X0, HEADER_Y),
        HEADER_LEFT,
        fontname="helv",
        fontsize=HEADER_FONT_SIZE,
        color=HEADER_COLOR,
    )
    # Right-align HEADER_RIGHT
    tw = fitz.get_text_length(HEADER_RIGHT, fontname="helv", fontsize=HEADER_FONT_SIZE)
    page.insert_text(
        (BODY_X1 - tw, HEADER_Y),
        HEADER_RIGHT,
        fontname="helv",
        fontsize=HEADER_FONT_SIZE,
        color=HEADER_COLOR,
    )
    # Header rule
    page.draw_line(
        (BODY_X0, HEADER_Y + 4),
        (BODY_X1, HEADER_Y + 4),
        color=RULE_COLOR,
        width=0.5,
    )

    # Footer rule + page number
    page.draw_line(
        (BODY_X0, RULE_Y_TOP),
        (BODY_X1, RULE_Y_TOP),
        color=RULE_COLOR,
        width=0.5,
    )
    pno = f"— {page_no} —"
    page.insert_textbox(
        fitz.Rect(0, FOOTER_Y - 4, A4_W, FOOTER_Y + 12),
        pno,
        fontname="djse", fontfile=fontmap.get(BODY_FONT, "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"),
        fontsize=FOOTER_FONT_SIZE, color=BODY_COLOR, align=1,
    )


def insert_paragraph(page, x, y, w, text, fontmap, font_key=BODY_FONT,
                     size=BODY_SIZE, leading=BODY_LEADING, color=BODY_COLOR, align=0):
    """Insert a wrapped paragraph; returns the y after the paragraph."""
    box = fitz.Rect(x, y, x + w, BODY_BOTTOM)
    if font_key in fontmap:
        # Use a unique 4-char alias for fontname when loading via fontfile
        alias = {
            BODY_FONT: "djse",
            BODY_FONT_BOLD: "djbo",
            BODY_FONT_ITAL: "djit",
        }.get(font_key, "djse")
        rv = page.insert_textbox(
            box, text,
            fontname=alias, fontsize=size, lineheight=leading / size,
            color=color, align=align, fontfile=fontmap[font_key],
        )
    else:
        # Fallback to built-in Helvetica/Times
        fallback = "helv" if "Helvetica" in font_key else ("tibo" if "Bold" in font_key else ("tiit" if "Italic" in font_key else "tiro"))
        rv = page.insert_textbox(
            box, text,
            fontname=fallback, fontsize=size, lineheight=leading / size,
            color=color, align=align,
        )
    # rv is the y-offset where text actually ended (negative = remaining height; positive doesn't exist).
    # pymupdf returns the unused vertical space at the bottom. If rv < 0, the text overflowed.
    # We approximate the used height as box.height - rv when rv >= 0.
    used = (box.height - rv) if rv >= 0 else box.height
    return y + used


def build_cover_page(out_doc: fitz.Document, cover_png: str, fontmap: dict):
    """Page 1: atlas-styled cover."""
    page = out_doc.new_page(width=A4_W, height=A4_H)
    # Fill page bg cream
    page.draw_rect(fitz.Rect(0, 0, A4_W, A4_H),
                   color=None, fill=(0xF7 / 255, 0xF6 / 255, 0xF2 / 255), overlay=False)
    draw_header_footer(page, 1, fontmap)

    # Title block — use insert_textbox so DejaVu glyphs (φ, ², ⁻, etc.) render correctly.
    title = "Vasilev-Pellis Constants"
    page.insert_textbox(
        fitz.Rect(0, 80, A4_W, 125),
        title,
        fontname="djbo", fontfile=fontmap.get(BODY_FONT_BOLD, "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf"),
        fontsize=TITLE_SIZE, color=BODY_COLOR, align=1,
    )
    sub = "A Three-Strand TRI-1 DNA Architecture under the Trinity S³AI DNA brand"
    page.insert_textbox(
        fitz.Rect(0, 128, A4_W, 152),
        sub,
        fontname="djit", fontfile=fontmap.get(BODY_FONT_ITAL, "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf"),
        fontsize=SUBTITLE_SIZE, color=BODY_COLOR, align=1,
    )
    authors = "Dmitrii Vasilev · Stergios Pellis · Scott Olsen"
    page.insert_textbox(
        fitz.Rect(0, 156, A4_W, 178),
        authors,
        fontname="djse", fontfile=fontmap.get(BODY_FONT, "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"),
        fontsize=AUTHORS_SIZE, color=BODY_COLOR, align=1,
    )
    anchor_line = "anchor identity:    φ² + φ⁻² = 3"
    page.insert_textbox(
        fitz.Rect(0, 182, A4_W, 204),
        anchor_line,
        fontname="djit", fontfile=fontmap.get(BODY_FONT_ITAL, "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf"),
        fontsize=AUTHORS_SIZE + 1, color=BODY_COLOR, align=1,
    )

    # Cover triptych image, sized to 80% of body width, centered, ~40% of page height
    img_w = (BODY_X1 - BODY_X0) * 0.96
    img_h = img_w * (941 / 1672)  # preserve aspect
    img_x = (A4_W - img_w) / 2
    img_y = 215
    page.insert_image(fitz.Rect(img_x, img_y, img_x + img_w, img_y + img_h),
                      filename=cover_png)
    caption = ("Cover plate — three strands of the framework: Input Constants "
               "(α, G, ℏ, c, mₑ), Symbolic Search (φ-based grammar), and "
               "Validation (falsification ledger).")
    insert_paragraph(page, BODY_X0, img_y + img_h + 8, BODY_X1 - BODY_X0,
                     caption, fontmap, font_key=BODY_FONT_ITAL,
                     size=BODY_SIZE - 0.5, align=1)

    # Dense body: abstract opener + Catalog42 contract — must match the atlas density
    body_y = img_y + img_h + 50
    intro = ("This article is the full-length presentation of the Pellis-Trinity / "
             "Vasilev-Pellis program in three strands. Strand I (Paper 1) is "
             "a constrained symbolic-compressibility study over a φ-structured "
             "monomial basis; Strand II (Paper 2) explores mechanism through the "
             "E8/Toda anchor, discrete scale invariance, and a candidate symbolic "
             "renormalisation group; Strand III (Paper 3) unifies the two via a "
             "hierarchical φ-expansion, a monomial-lattice grammar, and "
             "Koopman / transfer-operator dynamics. Built in the atlas house style "
             "for the tri article service, with reviewer-facing hardening preserved "
             "and the new Trinity S³AI DNA brand applied throughout.")
    body_y = insert_paragraph(page, BODY_X0, body_y, BODY_X1 - BODY_X0, intro, fontmap, align=4)
    body_y += 8

    # Catalog42 contract — bold heading + bullet list
    heading = "Catalog42 wording lock"
    if BODY_FONT_BOLD in fontmap:
        page.insert_text((BODY_X0, body_y + 12), heading,
                         fontname=BODY_FONT_BOLD, fontsize=HEADING_SIZE,
                         color=BODY_COLOR, fontfile=fontmap[BODY_FONT_BOLD])
    else:
        page.insert_text((BODY_X0, body_y + 12), heading,
                         fontname="tibo", fontsize=HEADING_SIZE, color=BODY_COLOR)
    body_y += 22
    bullets = ("42 declared formula IDs · 19 rows with closed-with-Qed numeric "
               "tolerance proofs in the flagship Coq import chain · 23 UnderRevision "
               "rows with explicit proof obligations · zero Admitted in the flagship "
               "chain (8 files) · 32 Admitted quarantined in 5 non-flagship files · "
               "ten source-level checker gates (G1–G10), all PASS · coqc not run in "
               "the present sandbox: this is a source-level audit, not a new "
               "compiler verdict. Bonferroni correction is bounded by min(1, 15) = 1.")
    insert_paragraph(page, BODY_X0, body_y, BODY_X1 - BODY_X0, bullets, fontmap, align=4)


def build_abstract_page(out_doc: fitz.Document, plate_png: str, fontmap: dict):
    """Page 2: atlas-styled abstract page."""
    page = out_doc.new_page(width=A4_W, height=A4_H)
    page.draw_rect(fitz.Rect(0, 0, A4_W, A4_H),
                   color=None, fill=(0xF7 / 255, 0xF6 / 255, 0xF2 / 255), overlay=False)
    draw_header_footer(page, 2, fontmap)

    # H1 heading
    title = ("Low-Complexity Algebraic Representations of Physical Constants: "
             "A Constrained Symbolic-Compressibility Study with a φ-Structured Basis")
    y = BODY_TOP
    rv = page.insert_textbox(
        fitz.Rect(BODY_X0, y, BODY_X1, y + 120),
        title,
        fontname="djbo", fontfile=fontmap.get(BODY_FONT_BOLD, "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf"),
        fontsize=18.0, lineheight=1.18, color=BODY_COLOR, align=0,
    )
    used = (120 - rv) if rv >= 0 else 120
    y += used + 6

    authors = "Dmitrii Vasilev · Stergios Pellis · Scott Olsen"
    page.insert_textbox(
        fitz.Rect(BODY_X0, y, BODY_X1, y + 18),
        authors,
        fontname="djse", fontfile=fontmap.get(BODY_FONT, "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"),
        fontsize=BODY_SIZE, color=BODY_COLOR, align=0,
    )
    y += 18
    version = "Preprint — May 2026 · Vasilev-Pellis Constants v22.1 (atlas-unified front-matter)"
    page.insert_textbox(
        fitz.Rect(BODY_X0, y, BODY_X1, y + 18),
        version,
        fontname="djse", fontfile=fontmap.get(BODY_FONT, "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"),
        fontsize=BODY_SIZE, color=BODY_COLOR, align=0,
    )
    y += 24

    page.insert_textbox(
        fitz.Rect(BODY_X0, y, BODY_X1, y + 20),
        "Abstract",
        fontname="djbo", fontfile=fontmap.get(BODY_FONT_BOLD, "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf"),
        fontsize=HEADING_SIZE + 1, color=BODY_COLOR, align=0,
    )
    y += 18

    # Long abstract — dense body block
    abstract = (
        "This paper studies whether the dimensionless physical constants of the "
        "Standard Model and the cosmological concordance model admit statistically "
        "significant compressibility within a constrained symbolic algebraic "
        "language generated by the basis {φ, π, e, 3}, where φ = (1 + √5)/2 is the "
        "golden ratio. The question is posed as a pre-registered constrained "
        "symbolic-compressibility problem, not as a derivation of physical "
        "constants.\n\n"
        "A finite-complexity grammar G_φ is defined via an ell^1-bounded exponent "
        "lattice, generating a polynomial-cardinality hypothesis class HC at each "
        "complexity level C. A target dataset T is drawn from PDG 2024 and CODATA "
        "2022 under a pre-registration freeze protocol. Significance is evaluated "
        "against three null models (log-uniform, permutation, randomized-grammar) "
        "with MDL-based and Bayesian model comparison. Multiple-testing correction "
        "uses Benjamini-Hochberg with Bonferroni bounded by min(1, 15) = 1.")
    insert_paragraph(page, BODY_X0, y, BODY_X1 - BODY_X0, abstract, fontmap, align=4)

    # Continue with a styled plate at the bottom for atlas-density
    img_w = (BODY_X1 - BODY_X0) * 0.88
    img_h = img_w * (619 / 1100)
    img_x = (A4_W - img_w) / 2
    img_y = BODY_BOTTOM - img_h - 50
    page.insert_image(fitz.Rect(img_x, img_y, img_x + img_w, img_y + img_h),
                      filename=plate_png)
    caption = ("Figure (fig-p1-open). Opening triptych — Seed Identity / Vesica Axis / "
               "Sprout. The trilogy opens from one identity into three papers.")
    insert_paragraph(page, BODY_X0, img_y + img_h + 6, BODY_X1 - BODY_X0,
                     caption, fontmap, font_key=BODY_FONT_ITAL,
                     size=BODY_SIZE - 0.5, align=1)


def main(argv):
    ap = argparse.ArgumentParser()
    ap.add_argument("--in", dest="src", required=True)
    ap.add_argument("--out", dest="dst", required=True)
    ap.add_argument("--cover", required=True)
    ap.add_argument("--plate2", required=True)
    args = ap.parse_args(argv)

    out = fitz.open()
    fontmap = register_fonts(out)
    print(f"font map: {list(fontmap.keys())}")

    build_cover_page(out, args.cover, fontmap)
    build_abstract_page(out, args.plate2, fontmap)

    # Append v22 pages 4..end (skipping the old sparse pages 1-3)
    src = fitz.open(args.src)
    out.insert_pdf(src, from_page=3, to_page=src.page_count - 1)

    # Metadata
    out.set_metadata({
        "title": "Vasilev-Pellis Constants (Trinity S³AI DNA, v22.1 unified-frontmatter full atlas)",
        "author": "gHashTag/trios",
        "subject": "Full PhD-style atlas article with unified atlas-styled front matter from page 1",
        "creator": "tri article (repo runner) — v22.1 atlas-unified front-matter rebuild",
        "producer": "pymupdf direct page render + content-stream rewrite of v21 atlas",
        "keywords": "Vasilev-Pellis, Trinity S3AI DNA, Catalog42, golden-balance, Olsen, Tier-D",
    })

    out.save(args.dst, garbage=4, deflate=True, clean=True)
    print(f"wrote {args.dst} ({out.page_count} pages)")


if __name__ == "__main__":
    main(sys.argv[1:])
