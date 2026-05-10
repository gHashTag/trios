# Flos Aureus PhD Monograph — Canonical Illustrations Manifest

**Version:** v6.3 (canonical lock, 2026-05-10)
**Anchor:** φ²+φ⁻²=3, DOI 10.5281/zenodo.19227877
**Author:** Dmitrii Vasilev <raoffonom@icloud.com>, ORCID 0009-0008-4294-6159

This manifest establishes the **canonical illustration set** for the gHashTag/trios PhD monograph. **All other illustration files in `docs/phd/assets/`, `assets/illustrations/`, and `assets/illustrations_v516/`, and any `chXX-*.png`, `img-XXX.png`, or `pdf_en_img-*.{png,jpg}` files MUST be removed** to prevent style drift.

## Series A — `assets/illustrations/` (34 PNG, Azbuka style)

First-generation Da Vinci codex × Azbuka Tridevyatogo Tsarstva triptychs. Three-cartouche layout with illuminated initials, fleurons, "TRINITY S3AI — the cognitive stack rooted in the identity phi⁻² + phi⁻² = 3" footer ribbon.

| # | Filename | Chapter |
|---|---|---|
| 00 | `00-monad.png` | The Monad |
| 01 | `01-golden-egg.png` | Golden Egg |
| 02 | `02-golden-cut.png` | Golden Cut |
| 03 | `03-golden-harvest.png` | Golden Harvest |
| 04 | `04-golden-scales.png` | Golden Scales |
| 05 | `05-golden-bridge.png` | Golden Bridge |
| 06 | `06-golden-mantissa.png` | Golden Mantissa |
| 07 | `07-golden-sprout.png` | Golden Sprout |
| 08 | `08-golden-crystal.png` | Golden Crystal |
| 09 | `09-golden-seal.png` | Golden Seal |
| 10 | `10-golden-bloom.png` | Golden Bloom |
| 11 | `11-vesica-piscis.png` | Vesica Piscis |
| 12 | `12-flower-of-life.png` | Flower of Life |
| 13 | `13-metatron-cube.png` | Metatron's Cube |
| 14 | `14-platonic-solids.png` | Platonic Solids |
| 15 | `15-kepler-solids.png` | Kepler Solids |
| 16 | `16-sacred-ratios.png` | Sacred Ratios |
| 17 | `17-golden-spiral.png` | Golden Spiral |
| 18 | `18-torus-geometry.png` | Torus Geometry |
| 19 | `19-fibonacci-tesselation.png` | Fibonacci Tesselation |
| 20 | `20-standard-model.png` | Standard Model |
| 21 | `21-quantum-field.png` | Quantum Field |
| 22 | `22-e8-symmetry.png` | E8 Symmetry |
| 23 | `23-gf16-algebra.png` | GF16 Algebra |
| 24 | `24-igla-architecture.png` | IGLA Architecture |
| 25 | `25-benchmarks.png` | Benchmarks |
| 26 | `26-data-analysis.png` | Data Analysis |
| 27 | `27-trinity-identity.png` | Trinity Identity (φ²+φ⁻²=3) |
| 28 | `28-momentum-algebra.png` | Momentum Algebra |
| 29 | `29-lucas-closure.png` | Lucas Closure |
| 30 | `30-golden-imagery.png` | Golden Imagery |
| 31 | `31-philosophy.png` | Mathematical Platonism |
| 32 | `32-conclusion.png` | Conclusion |
| 33 | `33-epilogue.png` | Epilogue |

## Series B — `assets/illustrations_v516/` (48 JPG, blueprint-with-axes style)

Second-generation v5.16 triptychs with three-axis coordinate frames (z, y, x) and per-panel φ-formulae. Used for premium chapter openers and appendix headers.

### Chapters (34 files, mirror Series A)

Same NN-slug naming as Series A; format `NN-slug.jpg`. One-to-one mapping with chapter `\include{chapters/NN-slug}` directives.

### Appendices (14 files)

| Appendix | Filename | Topic |
|---|---|---|
| A | `A-catalogue.jpg` | Catalogue of Solids |
| B | `B-falsification.jpg` | Popper Falsification |
| C | `C-golden-benchmark.jpg` | Golden Benchmark |
| D | `D-golden-mirror.jpg` | Golden Mirror Lexicon |
| E | `E-lexicon.jpg` | Trinity Lexicon |
| F | `F-coq-citation-map.jpg` | Coq Citation Map |
| F | `F-fpga-bitstream.jpg` | FPGA Bitstream |
| G | `G-data-availability.jpg` | Data Availability |
| H | `H-acm-ae-checklist.jpg` | ACM AE Checklist |
| H | `H-zenodo-doi.jpg` | Zenodo DOI |
| I | `I-xdc-pin-map.jpg` | XDC Pin Map |
| J | `J-troubleshooting.jpg` | Troubleshooting |
| K | `K-agent-memory.jpg` | Agent Memory |
| L | `L-pollen-channel.jpg` | Pollen Channel |

## Removal manifest (files to PURGE from repository)

The following file patterns must be **removed** from `gHashTag/trios` so they no longer pollute the build or confuse future contributors:

```
docs/phd/assets/chXX-*.png         # legacy chapter naming (pre-v5.3)
docs/phd/assets/img-XXX.png         # mass-extracted from old PDFs
docs/phd/assets/cover_*.png         # all cover variants except canonical chalkboard
assets/illustrations/v519-*.png     # rejected v5.19 series
assets/illustrations/pdf_en_img-*   # PDF extraction noise
assets/illustrations_v516/v516-ch_NN.jpg  # slug-renumbered duplicates (NOT theme-matched)
```

## graphicspath order (in `main.tex` and `main_ru.tex`)

```latex
\graphicspath{
  {assets/}                          % chalkboard cover only
  {../../assets/illustrations/}      % Series A (Azbuka PNG)
  {../../assets/illustrations_v516/} % Series B (v5.16 JPG with axes)
}
```

Chapter `\includegraphics{NN-slug}` resolves to PNG first, then JPG fallback. Use `\includegraphics{NN-slug.jpg}` explicitly to force v5.16 series for premium full-page plates.

---

**SHA-256 manifest:** see `MANIFEST.sha256` in this directory.
**License:** CC-BY 4.0, attribution to Dmitrii Vasilev (ORCID 0009-0008-4294-6159).
