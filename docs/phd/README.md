# PhD Monograph — GOLDEN SUNFLOWERS / Trinity S³AI / Flos Aureus

**Author:** Dmitrii Vasilev (ORCID 0009-0008-4294-6159)
**Anchor:** φ² + φ⁻² = 3
**DOI:** 10.5281/zenodo.19227877
**Defense:** 2026-06-15

---

## ⚠️ SINGLE SSOT — READ THIS FIRST

After the **2026-05-15 unification** there is **ONE AND ONLY ONE** source of truth for the monograph:

```
Postgres @ trolley.proxy.rlwy.net:52162/railway
└── schema: ssot
    ├── chapters    (93 rows, markdown in body_md column)   ← TEXT
    └── assets      (186 rows, PNG/JPG in bytea column)     ← IMAGES
```

**Everything else is GENERATED.** The PDF, all `.tex` files, and the figure-map are derived from this Postgres SSOT by the Rust pipeline `phd-md2pdf`.

---

## 🛑 CROWN LAWS (mandatory rules)

### R0 — SINGLE SSOT
The only source of truth is Postgres `phd-postgres-ssot`, schema `ssot`, two tables: `ssot.chapters` (markdown in `body_md`) and `ssot.assets` (images in `bytea`). Everything else is derived.

### R1 — NO LATEX SOURCES IN REPO
Storing `chapters/*.tex`, `appendix/*.tex`, `frontmatter/*.tex`, `main.tex`, `main_ru.tex`, or `figure-map.tex` as **sources** in this repository is forbidden. These files may exist only as build artefacts in a temporary directory like `/tmp/phd-md-build/`. Any PR adding `.tex` to `docs/phd/chapters|appendix|frontmatter` must be **rejected**.

### R2 — NO IMAGE FILES IN REPO
Directories `assets/illustrations/`, `assets/illustrations_v516/` and similar are deleted. Images are stored ONLY in `ssot.assets.bytes`. To add a new figure: `INSERT INTO ssot.assets` — never commit binary images to git.

### R3 — MARKDOWN ONLY
Chapter content is written and edited in `ssot.chapters.body_md` (CommonMark + LaTeX math). The legacy `body_latex` column has been **dropped**. Conversion to LaTeX happens at build time only, inside the Rust pipeline.

### R4 — RUST PIPELINE ONLY
The PDF is built by `phd-build-from-md` from the `phd-md2pdf` crate. No Python scripts, no manual `pdflatex` / `pandoc` invocations. Pipeline contract:
```
ssot.chapters + ssot.assets  →  /tmp/phd-md-build  →  out/main.pdf
```

### R5 — BIBLIOGRAPHY EXCEPTION (temporary)
`docs/phd/bibliography.bib` remains in the repo until a future migration to `ssot.bibliography`. All citations are routed through this file by the build pipeline.

### R6 — DEFENSE SLIDES EXCEPTION
`docs/phd/defense/` remains in the repo as an independent SSOT for the 2026-06-15 defense slides. Not subject to R0–R4.

---

## 📦 What lives in `docs/phd/` after unification

```
docs/phd/
├── README.md           ← this file
├── Makefile            ← thin wrapper calling phd-md2pdf
├── bibliography.bib    ← R5 exception
└── defense/            ← R6 exception (slides)
```

That is the **entire** PhD source tree on disk. Everything else lives in Postgres.

---

## 🔨 Building the PDF

```bash
export DATABASE_URL="postgresql://...@trolley.proxy.rlwy.net:52162/railway"
make pdf
# → out/main.pdf  (~969 pages, 182 figures, ~24 MB after gs /ebook compression)
```

### Renderer rules (image placement, keep-together, image-train ban)

The canonical rules for image placement, hero-panel anchoring, and
typography are defined in:

- **[../pdf-rendering.md](../pdf-rendering.md)** — `TRIOS_PHD_CANONICAL_PIPELINE`,
  `TRIOS_PHD_RENDERER_FIRST`, `TRIOS_PHD_STYLE_LOCK`,
  `TRIOS_PHD_NO_IMAGE_TRAIN`.

Headline: hero panels are anchored to the nearest substantive heading
via `\Needspace*{0.58\textheight}` (soft keep-together). Hard
`\clearpage` before sections is forbidden — it produced short
title-only pages and was rejected by QA.

---

## 📜 Audit trail

- Pre-unification backup: `phd_ssot_backup_20260515_*.dump` (265 MB) + `phd_repo_backup_20260515_*.tar.gz` (471 MB)
- Pre-unification branch: `archive/pre-ssot-cleanup-20260515_*`
- Duplicate audit before cleanup: `phd_audit_duplicates.md`
- Unification plan: `phd_ssot_unification_plan.md`
