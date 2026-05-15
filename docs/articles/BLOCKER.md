# Build blocker — `tri article` subcommand not yet implemented

This file documents why `tri article build pellis-trinity-full --pdf` /
`--html` / `qa` did **not** run in the commit that introduced this
directory, and what the next environment needs to do to complete the
build.

## Observed state

In the current repository:

- `crates/tri-cli/` is the Tailscale-funnel + trios-server launcher CLI
  (`tri start | stop | status`). It does **not** define an `article`
  subcommand. (See `crates/tri-cli/src/main.rs`.)
- `crates/trios-cli/` defines the trios research-loop CLI
  (`agent | commit | dash | gates | ...`). It also does **not** define an
  `article` subcommand. (See `crates/trios-cli/src/cmd/`.)
- There is no `tri article` binary on `PATH` in the sandbox, and `cargo`
  itself is not installed in the sandbox, so even the `cargo run -p
  tri-cli -- article ...` fallback cannot be exercised here.

The article therefore cannot be built end-to-end through the canonical
`tri article` service from this sandbox. Per the agent instructions,
"if build is blocked, commit source changes locally and provide blocker
details and exact commands for an environment with Rust/Cargo." That is
what this commit does.

## What is committed upstream

- `docs/articles/pellis-trinity-full/article.toml` — article descriptor
  (preset, language, integration label, render flags).
- `docs/articles/pellis-trinity-full/body/*.md` — body in 13 ordered
  sections, with **all reviewer corrections integrated inline**:
  - Catalog42 wording lock (mapped proof-obligation catalogue, 42 declared
    / 19 closed-with-Qed / 23 UnderRevision / 0 flagship `Admitted.` /
    32 quarantined; source-level audit only).
  - L01, L02, L03, Q03, Q05 downgraded to UnderRevision with measured
    relative errors and explicit "do not cite as verified" wording.
  - Bonferroni correction capped: $p_{\text{Bonf}} = \min(1, n p) = 1$
    when $n p \approx 15$.
  - Proposition 8.2 rewritten with two distinct conventions ($R = 0$
    vs. $\mu_T = -\infty$).
  - All `[link]` placeholders replaced by real URLs (NIST CODATA, CODATA
    2022 PDF, PDG 2024 QCD, AI Feynman, Grünwald MDL).
  - Wilson & Kogut cited as *Phys. Rep.* **12**, 75–199 (1974).
  - Sacred ALU and v21-integration-label sentences restored (no more
    truncation).
- `docs/articles/pellis-trinity-full/figures/manifest.json` — vector
  figure manifest with `regenerate_required: true` and explicit
  `must_not_use` rules for legacy pages 17, 35, 51. The renderer must
  redraw these as vector PDF triptychs / banners / diagrams **in the
  same house style** instead of substituting plain replacement pages.
- `docs/articles/pellis-trinity-full/presets/phdstyle-atlas.toml` — house
  style (A4, colours, fonts, label minimums, annotation policy).
- `docs/articles/pellis-trinity-full/qa/pellis-trinity-full.qa.toml` —
  QA gates (forbidden phrases, required phrases, row downgrades,
  annotation policy with `expected_annots_when_no_links = 0`, figure
  rules, required citation anchors, numeric sanity).

## What the next environment must do

1. Install Rust (`rustup default stable`) so `cargo` is on `PATH`.
2. Implement the `article` subcommand on the `tri` CLI. The canonical
   layout is already in place; the subcommand needs to:
   - `list`: scan `docs/articles/*/article.toml` and print
     `{slug, title, version}`.
   - `presets`: scan `docs/articles/*/presets/*.toml` and print
     `{preset.name, description}`.
   - `build <slug> --pdf`: render `body/*.md` in lexical order with the
     preset, render figures from `figures/manifest.json` as vector PDF
     (no rasterized text), and write the PDF to
     `docs/articles/<slug>/build/<slug>.pdf`. Final `/Annots` count must
     be 0 unless `render.emit_links = true` AND real hyperlinks exist.
     No highlight / text-markup / comment annotations under any setting.
   - `build <slug> --html`: render to
     `docs/articles/<slug>/build/<slug>.html`.
   - `qa <slug>`: load `qa/<slug>.qa.toml`, run `pdftotext` over the PDF,
     check `forbidden_phrases`, `required_phrases`, `catalog42_row_status`,
     `annotations`, `figures`, `references`, `numerics`, plus the
     external tool gates. Exit non-zero on any failure.
3. Run the canonical commands:
   ```bash
   tri article list
   tri article presets
   tri article build pellis-trinity-full --pdf
   tri article build pellis-trinity-full --html
   tri article qa    pellis-trinity-full
   qpdf  --check docs/articles/pellis-trinity-full/build/pellis-trinity-full.pdf
   pdfinfo       docs/articles/pellis-trinity-full/build/pellis-trinity-full.pdf
   pdfimages -list docs/articles/pellis-trinity-full/build/pellis-trinity-full.pdf
   pdftotext     docs/articles/pellis-trinity-full/build/pellis-trinity-full.pdf - \
     | grep -E -i '42/42|UnderRevision|Bonferroni|Admitted|\[link\]|Wilson|Kogut|muT'
   ```
4. Confirm `tri article qa pellis-trinity-full` exits zero before
   opening a PR.

## What was explicitly avoided

- No `pypdf` page replacement / overlay of plain ReportLab pages.
- No removal of figures by blanking pages.
- No manual PDF surgery as the publication path.
- No orange highlight / comment annotations on the final PDF.
- No `42/42 Coq verified` wording.
- No `[link]` placeholders in the body.
- No L01 / L02 / L03 / Q03 / Q05 cited as verified.
