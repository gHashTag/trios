# Figures — pellis-trinity-full

This directory holds the figure manifest consumed by the `tri article`
PhD-style atlas renderer. The renderer must produce vector PDF figures
from this manifest, with real PDF text labels in English, in the house
style defined in `manifest.json`.

The renderer must **not** substitute plain ReportLab replacement pages for
figures that fail QA. Instead it must fail the build with a clear error
referencing the offending figure id, so the source can be fixed.

Three legacy pages required upstream regeneration in the v21 audit:

- legacy page 17 → `fig-grammar-acceptance` (pseudo-Latin / microtext)
- legacy page 35 → `fig-s3ai-banner`         (garbled AI text)
- legacy page 51 → `fig-pellis-ladder`       (rasterized stylized text, dense labels)

All three are marked `regenerate_required: true` in `manifest.json`. The
fix is to draw them as vector diagrams in the same house style, not to
remove or blank them.
