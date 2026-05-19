# `docs/articles/_runner/`

Repo-native build + QA runner for `docs/articles/<slug>/` article sources.

This runner is the working backend for the `tri article` subcommand
(`crates/tri-cli`). The Rust subcommand exec's into this Node runner so
that the same renderer is used whether invoked as `tri article ...`,
`cargo run -p tri-cli -- article ...`, or directly as
`node docs/articles/_runner/src/main.mjs ...`.

## Why a Node runner instead of pure Rust

The article pipeline needs Markdown → HTML → PDF with WCAG-AA labels,
real PDF text, and CSS-grade typography. The mature options on the
build host are WeasyPrint (Python) and headless Chromium. Both are
exec'd as external tools; the runner itself is small and only handles:

1. Parsing `article.toml` + the preset TOML.
2. Concatenating `body/*.md` in lexical order and rendering through
   `markdown-it`.
3. Wrapping in a single house-style HTML template that carries the
   `[render.header]` strings (`Vasilev-Pellis Constants` /
   `Trinity S³AI DNA`) on every page.
4. Spawning `weasyprint` to produce the PDF.
5. Running QA gates from `qa/<slug>.qa.toml` over the rendered HTML +
   PDF (forbidden / required phrases, `qpdf --check`, `/Annots` audit).

## Commands

```bash
node docs/articles/_runner/src/main.mjs list
node docs/articles/_runner/src/main.mjs presets
node docs/articles/_runner/src/main.mjs build pellis-trinity-full --pdf
node docs/articles/_runner/src/main.mjs build pellis-trinity-full --html
node docs/articles/_runner/src/main.mjs qa    pellis-trinity-full
```

The Rust subcommand mirrors this surface:

```bash
cargo run -p tri-cli -- article list
cargo run -p tri-cli -- article presets
cargo run -p tri-cli -- article build pellis-trinity-full --pdf
cargo run -p tri-cli -- article build pellis-trinity-full --html
cargo run -p tri-cli -- article qa    pellis-trinity-full
```

## Required system tools

- `node` ≥ 18
- `weasyprint` (for `--pdf`)
- `qpdf` (for QA `qpdf --check`)
- `pdftotext` from poppler (for QA grep of PDF body text)

## L1 compliance

This runner is TypeScript-style ESM JavaScript (`.mjs`). No `.sh`
files are introduced (Constitutional L1).
