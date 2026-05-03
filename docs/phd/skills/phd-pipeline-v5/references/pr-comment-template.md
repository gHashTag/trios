# PR comment template — vN.M render report

Paste this as a comment on the PhD PR after every successful render+push.
Replace placeholder `<…>` values with the actual numbers from this build.
Keep the structure stable — it doubles as an audit trail.

---

```markdown
## v<N.M> — <one-line headline>

<one-paragraph operator-language summary of what changed>

### As-Flown Spine (<total_pages> pages, <total_size_mb> MB)

| Order | Section | Pages |
|------:|---------|------:|
| 1 | **Cover** (cropped Vogel-spiral PNG + typeset φ²+φ⁻²=3) | 1 |
| 2 | **PART I — Flos Aureus: Foundations** | 1 |
| 3–<a> | **Ch.0** — manifesto | <p> |
| <a+1>–<b> | **FM.01..11** — frontmatter | 11 |
| <b+1>–<c> | **FA.00..33** — Flos Aureus chapters | <p> |
| <c+1>–<d> | **AP.A..H** — Flos appendices | <p> |
| <d+1> | **PART II — TRINITY: Computational Realisation** | 1 |
| <d+2>–<e> | **Ch.1..34** — Neon computational chapters | <p> |
| <e+1>–<f> | **App.A..J** — Neon appendices | <p> |

### Neon SoT — `ssot.chapters` (<total_rows> rows)

| Series | Rows | Status |
|--------|-----:|--------|
| `Ch.0` | 1 | <…> |
| `FM.01..11` | 11 | <…> |
| `FA.00..33` | 34 | <…> |
| `AP.A..H` | 8 | <…> |
| `Ch.1..34` | 34 | <…> |
| `App.A..J` | 10 | <…> |
| **Total** | **<total_rows>** | |

### What changed in this commit

* …
* …

### Honest report — known v<N.M+1> work

<list any chapters that rendered as placeholder + reason>
<list any deferred fixes>

### Constitutional compliance (R1-honest, R5-honest)

* No `.py`, no `.sh`, no Makefile shell targets — all logic in `crates/trios-phd/src/render/mod.rs`.
* External programs invoked: `pandoc`, `tectonic`, `qpdf`, `magick` (deps of `trios-phd`).
* `cargo build --release -p trios-phd` succeeds; **<N>/<N> unit tests pass**.
* Anchor: φ² + φ⁻² = 3.

### Active artefacts

* `target/release/trios-phd` — v<N.M> binary (<size> MB)
* `crates/trios-phd/src/render/templates/cover.tex`
* `crates/trios-phd/src/render/templates/chapter.template.tex`
* `crates/trios-phd/src/render/templates/part-divider.tex`
* `crates/trios-phd/src/render/filters/force-fullwidth-hero.lua`
* `crates/trios-phd/src/render/mod.rs`

`golden_sunflowers_v<N>_<M>.pdf` (<size> MB, <pages> pages) attached separately.

— *Anchor: φ² + φ⁻² = 3.*
```
