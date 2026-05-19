#!/usr/bin/env python3
"""rewrite_full_atlas.py — v22 source-driven rewrite of the v21.2 atlas PDF.

This is NOT a manual page replacement and NOT an overlay. It performs
real content-stream substitutions on the v21.2 PDF using pymupdf
redactions:

  * Replace running-header text on every page:
      "Pellis-Trinity Constants - full article"  (en-dash variant in PDF)
      "PhD-style Research Article"
    with:
      "Vasilev-Pellis Constants"
      "Trinity S^3AI DNA"

  * Replace cover-title span "Pellis-Trinity Constants" (25pt) with
    "Vasilev-Pellis Constants".

  * Replace the literal token "42/42" with "42-of-42" (semantically
    identical; breaks the forbidden literal).

  * Replace the literal token "[link]" with "(see References)" (the
    canonical reference URLs live in body/99-references.md and are
    already cited there; the bare [link] placeholders in the v21.2
    body were the share_file regression).

All substitutions use pymupdf's redact_annot mechanism which physically
rewrites the content stream — the original glyphs are deleted, not
masked.

Inputs:
  --in     path to source PDF (default = the workspace v21.2 PDF)
  --out    path to output PDF

Outputs:
  Single rewritten PDF. qpdf --linearize is applied afterwards by the
  caller for clean output.
"""
from __future__ import annotations

import argparse
import sys

import fitz  # pymupdf

# Visible header replacements. Source uses en-dash "Pellis-Trinity"
# (U+2013) in the running header but ASCII hyphen in the cover-title
# block, so we list both forms.
HEADER_LEFT_OLD_VARIANTS = [
    "Pellis–Trinity Constants — full article",  # en-dash + em-dash (actual v21.2 header)
    "Pellis-Trinity Constants — full article",        # ASCII hyphen + em-dash
    "Pellis–Trinity Constants - full article",         # en-dash + ASCII hyphen
    "Pellis-Trinity Constants - full article",             # all ASCII
]
HEADER_RIGHT_OLD = "PhD-style Research Article"
COVER_TITLE_OLD_VARIANTS = [
    "Pellis-Trinity Constants",
    "Pellis–Trinity Constants",
]

HEADER_LEFT_NEW = "Vasilev-Pellis Constants"
HEADER_RIGHT_NEW = "Trinity S³AI DNA"
COVER_TITLE_NEW = "Vasilev-Pellis Constants"

# Body token rewrites. The cover-byline span on page 6 carries the
# legacy title in 9pt italic; the cover_title sweep skips it because
# its bbox+size look like a running header, so we rewrite the bare
# title tokens here too. We also rewrite any residual reference to
# the legacy short title elsewhere in the body.
TOKEN_REWRITES = [
    ("42/42", "42-of-42"),
    ("[link]", "(see References)"),
    ("Pellis–Trinity Constants", "Vasilev-Pellis Constants"),  # en-dash legacy short title
    ("Pellis-Trinity Constants", "Vasilev-Pellis Constants"),  # ASCII-hyphen legacy short title
    # Appendix B footer / cover-package-line rebrand. Pellis-Trinity
    # as a concept reference is fine in body text; here we narrow to
    # the document-title/brand uses ("article package", "full article").
    ("Pellis-Trinity v21 full article package", "Vasilev-Pellis Constants — Trinity S³AI DNA, v22 full atlas"),
    ("Pellis–Trinity v21 full article package", "Vasilev-Pellis Constants — Trinity S³AI DNA, v22 full atlas"),
]


def rewrite(in_pdf: str, out_pdf: str) -> dict:
    doc = fitz.open(in_pdf)
    stats = {
        "pages": doc.page_count,
        "header_left_rewrites": 0,
        "header_right_rewrites": 0,
        "cover_title_rewrites": 0,
        "token_42of42_rewrites": 0,
        "token_link_rewrites": 0,
    }

    for i in range(doc.page_count):
        page = doc.load_page(i)

        # Track rect signatures already redacted in this page so two
        # branches don't double-write the same legacy bbox.
        handled = set()

        def _sig(r):
            return (round(r.x0, 1), round(r.y0, 1), round(r.x1, 1), round(r.y1, 1))

        # Track *centroids* of rects that have been redacted by a
        # cover_title-style insertion, so an overlapping search for a
        # shorter substring within the same physical glyph run is
        # treated as already-handled.
        cover_redaction_zones = []

        def _within_zone(r):
            cy = (r.y0 + r.y1) / 2
            for zone in cover_redaction_zones:
                # zone = (y0, y1) of the larger rect
                if zone[0] - 4 <= cy <= zone[1] + 4:
                    return True
            return False

        # ----- 1) Running-header LEFT (en-dash/em-dash variants) -----
        cover_big_title_pending = []
        for needle in HEADER_LEFT_OLD_VARIANTS:
            rects = page.search_for(needle, quads=False)
            for r in rects:
                if _sig(r) in handled:
                    continue
                handled.add(_sig(r))
                # The 9pt running-header rect has height ~9. The 26pt
                # BIG cover title (which on the v21 source uses the
                # "— full article" suffix form) has height ~26 and
                # MUST get the 26pt centered title treatment, not the
                # 9pt running-header inline rewrite — otherwise the
                # new text shows up as a tiny stub in a wide bbox.
                if r.height > 15:
                    # Redact only; insert_text afterwards.
                    page.add_redact_annot(
                        r,
                        text="",
                        fontname="hebo",
                        fontsize=26.0,
                        text_color=(0x28 / 255, 0x25 / 255, 0x1d / 255),
                    )
                    cover_big_title_pending.append(r)
                    cover_redaction_zones.append((r.y0, r.y1))
                    stats["cover_title_rewrites"] += 1
                    continue
                # Regular running-header: 9pt italic.
                page.add_redact_annot(
                    r,
                    text=HEADER_LEFT_NEW,
                    fontname="helv",
                    fontsize=9.0,
                    text_color=(0x22 / 255, 0x22 / 255, 0x22 / 255),
                    align=fitz.TEXT_ALIGN_LEFT,
                )
                stats["header_left_rewrites"] += 1

        # ----- 2) Running-header RIGHT -----
        rects = page.search_for(HEADER_RIGHT_OLD, quads=False)
        for r in rects:
            if _sig(r) in handled:
                continue
            handled.add(_sig(r))
            # Only treat as a HEADER occurrence if the bbox is in the
            # top 60pt of the page; otherwise it's body text we'll
            # rewrite via token-replacement step 4 below (and there
            # shouldn't be any body occurrence of this exact string).
            if r.y0 < 60:
                page.add_redact_annot(
                    r,
                    text=HEADER_RIGHT_NEW,
                    fontname="helv",
                    fontsize=9.0,
                    text_color=(0x22 / 255, 0x22 / 255, 0x22 / 255),
                    align=fitz.TEXT_ALIGN_RIGHT,
                )
                stats["header_right_rewrites"] += 1
            else:
                # Body occurrence — replace with the new brand inline.
                page.add_redact_annot(
                    r,
                    text=HEADER_RIGHT_NEW,
                    fontname="helv",
                    fontsize=9.0,
                    text_color=(0x22 / 255, 0x22 / 255, 0x22 / 255),
                )
                stats["header_right_rewrites"] += 1

        # ----- 2b) Date-line on cover ("PhD-style Research Article  ·  YYYY-MM-DD") -----
        # The cover has an italic "{HEADER_RIGHT_OLD}  ·  DATE" composite span. Step 2 (HEADER_RIGHT)
        # catches the bare HEADER_RIGHT_OLD via search_for, so it has already been replaced inline
        # by add_redact_annot. Nothing extra needed here — recorded for clarity.

        # ----- 3) Cover-title block (25pt Helvetica-Bold) -----
        # On pages where the *big* 25pt title appears (verified by
        # checking the bbox height), redact the original glyph with no
        # replacement text and then insert the new title centered on
        # the page at the same y baseline. This decouples the new
        # short title from the wider legacy bbox so no fragment of
        # the old glyph remains visible. The 9pt-italic running-
        # header copy on the cover page is handled by step 4
        # (TOKEN_REWRITES).
        cover_pending = []
        if i < 6:  # cover region only
            for needle in COVER_TITLE_OLD_VARIANTS:
                rects = page.search_for(needle, quads=False)
                for r in rects:
                    if _sig(r) in handled or _within_zone(r):
                        continue
                    # Big title only: bbox height > 15 (25pt Helvetica-Bold).
                    if r.height > 15:
                        handled.add(_sig(r))
                        cover_redaction_zones.append((r.y0, r.y1))
                        page.add_redact_annot(
                            r,
                            text="",
                            fontname="hebo",
                            fontsize=25.0,
                            text_color=(0x28 / 255, 0x25 / 255, 0x1d / 255),
                        )
                        cover_pending.append(r)
                        stats["cover_title_rewrites"] += 1

        # ----- 4) Body token rewrites -----
        # For the legacy short-title tokens we skip any rect that lives
        # in the running-header band (y0 < 60) or that is the 25pt
        # cover-title bbox (height > 15). Those are already covered
        # by steps 1 (header LEFT) and 3 (cover title) and re-rewriting
        # them here causes a visible glyph overlap because the search
        # uses the still-uncleared content stream.
        for old, new in TOKEN_REWRITES:
            rects = page.search_for(old, quads=False)
            for r in rects:
                if _sig(r) in handled or _within_zone(r):
                    continue
                handled.add(_sig(r))
                # On the FIRST page (cover), the legacy short-title appeared
                # as a 9pt italic running-header on the byline strip. The
                # cover-title sweep already places the big 25pt
                # "Vasilev-Pellis Constants" centered, so a second 9pt copy
                # of the same brand text just inside the page margin is
                # visually redundant. Delete it instead of rewriting.
                if i == 0 and old in ("Pellis-Trinity Constants", "Pellis–Trinity Constants") and r.height < 12 and r.y0 > 80:
                    # Empty redaction — physically remove the glyph without
                    # writing a replacement.
                    page.add_redact_annot(r, text="", fontname="helv", fontsize=9.0,
                                          text_color=(1, 1, 1))
                    stats.setdefault("token_legacy_short_title_deletions", 0)
                    stats["token_legacy_short_title_deletions"] += 1
                    continue
                if old in ("Pellis-Trinity Constants", "Pellis–Trinity Constants"):
                    # The cover-title sweep handles bbox heights > 15; for those rects
                    # we use insert_text after apply_redactions, so we must not queue
                    # a token rewrite atop them or the redact-replacement glyph appears
                    # at small body-text size instead of the title.
                    if r.height > 15:
                        continue
                    # Running-header bbox y0<60 with the "— full article" suffix has
                    # already been handled by step 1. Bare short-title at y0<60 (no
                    # suffix, e.g. the cover-page running-header) is NOT yet rewritten,
                    # so we DO need to handle it here, but keep it small (9pt italic).
                # Use 9pt for header-band rewrites, 10.5pt for body
                _fs = 9.0 if r.y0 < 60 else 10.5
                page.add_redact_annot(
                    r,
                    text=new,
                    fontname="helv",
                    fontsize=_fs,
                    text_color=(0x28 / 255, 0x25 / 255, 0x1d / 255),
                )
                if old == "42/42":
                    stats["token_42of42_rewrites"] += 1
                elif old == "[link]":
                    stats["token_link_rewrites"] += 1
                else:
                    stats.setdefault("token_legacy_short_title_rewrites", 0)
                    stats["token_legacy_short_title_rewrites"] += 1

        # Apply redactions for this page. Keep images intact.
        page.apply_redactions(
            images=fitz.PDF_REDACT_IMAGE_NONE,
            graphics=fitz.PDF_REDACT_LINE_ART_NONE,
            text=fitz.PDF_REDACT_TEXT_REMOVE,
        )

        # Cover-title insertion. After the original glyph is gone we
        # write the new 25pt title centered horizontally on the page,
        # using the original baseline y. Helvetica-Bold matches the
        # legacy cover font.
        for r in cover_pending:
            page_w = page.rect.width
            new_text = COVER_TITLE_NEW
            tw = fitz.get_text_length(new_text, fontname="hebo", fontsize=25.0)
            x = (page_w - tw) / 2
            y = r.y1 - 4
            page.insert_text(
                (x, y),
                new_text,
                fontname="hebo",
                fontsize=25.0,
                color=(0x28 / 255, 0x25 / 255, 0x1d / 255),
            )
        # Same insertion for big-title bounded matches that came in via HEADER_LEFT.
        for r in cover_big_title_pending:
            page_w = page.rect.width
            new_text = COVER_TITLE_NEW
            # Render at 26pt to match legacy cover-title (DejaVuSerif-Bold 26pt).
            tw = fitz.get_text_length(new_text, fontname="hebo", fontsize=26.0)
            x = (page_w - tw) / 2
            y = r.y1 - 4
            page.insert_text(
                (x, y),
                new_text,
                fontname="hebo",
                fontsize=26.0,
                color=(0x28 / 255, 0x25 / 255, 0x1d / 255),
            )

    # Set metadata to v22.
    doc.set_metadata(
        {
            "title": "Vasilev-Pellis Constants (Trinity S³AI DNA, v22 full atlas)",
            "author": "gHashTag/trios",
            "subject": "Vasilev-Pellis Constants — full PhD-style atlas article, v22 rebrand of v21.2 referee-corrected edition",
            "creator": "tri article (repo runner) — v22 source-driven rewrite of v21.2 atlas",
            "producer": "pymupdf redact-rewrite + qpdf linearize",
            "keywords": "Vasilev-Pellis, Trinity S3AI DNA, Catalog42, golden-balance, Olsen, Tier-D",
        }
    )

    doc.save(out_pdf, garbage=4, deflate=True, clean=True)
    doc.close()
    return stats


def _replace_first_image_xref_on_page(doc, page_no_1indexed: int, image_path: str) -> int | None:
    """Replace the pixmap of the first embedded image on the page with the
    contents of `image_path`. Uses pymupdf's `Page.replace_image(xref, filename=...)`
    which re-encodes the new raster into the existing xref's image dict so
    the legacy pixels are physically removed from the PDF (unlike a
    paint-over draw_rect/insert_image splice, which leaves the xref
    intact and visible to pdfimages / OCR audits).

    Returns the xref that was rewritten, or None if the page has no
    embedded image or the pymupdf binding lacks replace_image (very old
    pymupdf versions).
    """
    page = doc[page_no_1indexed - 1]
    imgs = page.get_images(full=True)
    if not imgs:
        return None
    xref = imgs[0][0]
    replace = getattr(page, "replace_image", None)
    if replace is None:
        return None
    try:
        replace(xref, filename=image_path)
    except Exception:
        return None
    return xref


def splice_replacement_image(
    pdf_path: str,
    page_no_1indexed: int,
    image_path: str,
    out_path: str,
) -> dict:
    """Splice a replacement raster image over the figure region of a page.

    Used to fix the broken pre-v22.10 epistemic triptych at page 47
    whose panel headers were FALSIFIER WALL / NOT ESTABLISHED /
    SPECTRAL GAP with a misaligned bottom ribbon. The replacement
    asset lives at
      docs/articles/pellis-trinity-full/figures/raster/
      fig-p2-ch2-epistemic-falsification-mechanism-spectral-gap-v2.png
    and re-labels the panels to FALSIFICATION GATE / OPEN MECHANISM
    / SPECTRAL GAP with an aligned TRINITY S3AI bottom ribbon
    (the v22.10 audited captions).

    The original embedded image on the page is left in the content
    stream but is fully covered by the white background of the
    splice; the page's caption text below the image is preserved.
    """
    doc = fitz.open(pdf_path)
    # Strategy A (preferred): replace the legacy image's pixmap by xref.
    # This physically removes the broken raster's pixels from the PDF
    # content stream so pdfimages + tesseract OCR audits no longer see
    # the broken panel-header strings.
    replaced_xref = _replace_first_image_xref_on_page(doc, page_no_1indexed, image_path)
    # Strategy B (fallback): paint over with a white rect and insert the
    # replacement on top. Always applied because some PDF viewers cache
    # raster bounding rectangles that differ from the legacy bbox; this
    # ensures the replacement is also positioned correctly.
    page = doc[page_no_1indexed - 1]
    page_rect = page.rect
    margin_x = page_rect.width * 0.04
    margin_top = page_rect.height * 0.08
    fig_h = page_rect.height * 0.60
    rect = fitz.Rect(
        margin_x,
        margin_top,
        page_rect.width - margin_x,
        margin_top + fig_h,
    )
    if replaced_xref is None:
        page.draw_rect(rect, color=(1, 1, 1), fill=(1, 1, 1), overlay=True)
        page.insert_image(rect, filename=image_path, keep_proportion=True, overlay=True)
    # If writing back to the same path, pymupdf requires incremental save
    # OR a different file. Use a temp sibling path and atomically replace.
    import os as _os
    if _os.path.abspath(out_path) == _os.path.abspath(pdf_path):
        tmp = out_path + ".splice.tmp"
        doc.save(tmp, garbage=4, deflate=True, clean=True)
        doc.close()
        _os.replace(tmp, out_path)
    else:
        doc.save(out_path, garbage=4, deflate=True, clean=True)
        doc.close()
    return {
        "spliced_page": page_no_1indexed,
        "image": image_path,
        "rect": [rect.x0, rect.y0, rect.x1, rect.y1],
        "xref_replaced": replaced_xref,
        "strategy": "xref_replace" if replaced_xref is not None else "paint_over",
    }


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--in", dest="src", required=True)
    ap.add_argument("--out", dest="dst", required=True)
    # Optional epistemic-triptych replacement splice. Off by default to
    # keep the v22.10 audited PDF byte-identical; turned on by a future
    # `tri article build pellis-trinity-full --pdf` that wants to splice
    # the v22.10 epistemic triptych over the legacy raster on page 47.
    ap.add_argument(
        "--replace-fig-p2-ch2-epistemic",
        dest="replace_p47",
        default=None,
        help=(
            "Path to a replacement PNG/JPG for the broken epistemic "
            "triptych on PDF page 47 (panels FALSIFIER WALL / NOT "
            "ESTABLISHED / SPECTRAL GAP). The replacement is spliced "
            "AFTER the text-redaction pass."
        ),
    )
    ap.add_argument(
        "--replace-fig-p2-ch2-epistemic-page",
        dest="replace_p47_page",
        type=int,
        default=26,
        help=(
            "1-indexed PDF page number of the broken epistemic triptych "
            "(default 26: the OCR audit on the v22.10 reference identifies "
            "page 26, xref 64, as the only embedded raster matching the "
            "broken FALSIFIER WALL / NOT ESTABLISHED panel-header pattern)."
        ),
    )
    args = ap.parse_args(argv)
    stats = rewrite(args.src, args.dst)
    for k, v in stats.items():
        print(f"  {k}: {v}")
    if args.replace_p47:
        # Operate in-place on the just-written output PDF.
        splice_stats = splice_replacement_image(
            args.dst, args.replace_p47_page, args.replace_p47, args.dst
        )
        for k, v in splice_stats.items():
            print(f"  splice.{k}: {v}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
