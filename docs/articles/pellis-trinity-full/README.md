# pellis-trinity-full

Canonical article source for the **Pellis-Trinity PhD-Style Atlas (v21, style-safe edition)**, rendered by the `tri article` service.

This directory is the upstream source of truth. All reviewer corrections
land here, not in a post-processed PDF.

## Layout

```
docs/articles/pellis-trinity-full/
├── README.md                  # this file
├── article.toml               # article metadata + section ordering for `tri article`
├── body/                      # ordered markdown sections (rendered in lexical order)
│   ├── 00-frontmatter.md
│   ├── 01-abstract.md
│   ├── 02-proof-status-lock.md
│   ├── 03-strand-i-mathematics.md
│   ├── 04-strand-ii-cognition.md
│   ├── 05-strand-iii-language-hardware.md
│   ├── 06-corrected-claims-table.md
│   ├── 07-catalog42-row-status.md
│   ├── 08-statistical-multiplicity.md
│   ├── 09-proposition-8-2.md
│   ├── 10-reviewer-risk-register.md
│   ├── 11-followups.md
│   └── 99-references.md
├── figures/
│   ├── manifest.json          # figure regeneration spec (in-source, vector-only)
│   └── README.md
├── presets/
│   └── phdstyle-atlas.toml    # PhD-style atlas preset (house style)
├── qa/
│   └── pellis-trinity-full.qa.toml  # QA gates (greps, annotation count, etc.)
└── build/                     # outputs from `tri article build` go here
```

## Canonical commands

```bash
tri article list
tri article presets
tri article build pellis-trinity-full --pdf
tri article build pellis-trinity-full --html
tri article qa   pellis-trinity-full
```

If `tri` is not installed:

```bash
cargo run -p tri-cli -- article list
cargo run -p tri-cli -- article presets
cargo run -p tri-cli -- article build pellis-trinity-full --pdf
cargo run -p tri-cli -- article build pellis-trinity-full --html
cargo run -p tri-cli -- article qa   pellis-trinity-full
```

## Style invariant

The PhD-style atlas visual language is set by `presets/phdstyle-atlas.toml`.

Do **not** patch the final PDF by replacing pages with plain ReportLab pages, do **not**
overlay blank pages to hide figures, and do **not** use ad-hoc `pypdf` merge/replace as
the publication path. Fix problems upstream in this directory.
