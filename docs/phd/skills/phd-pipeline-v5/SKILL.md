---
name: phd-pipeline-v5
description: "Build the unified Flos Aureus + Neon-computational PhD monograph (v5.x) from the Neon SoT (`ssot.chapters`). Use when the operator says 'render PhD', 'rebuild monograph', 'обнови PhD', 'render unified', 'v5.x', mentions Flos Aureus chapters (FA.NN), Neon chapters (Ch.N / App.X), the cover (cropped Vogel-spiral + φ²+φ⁻²=3 plate), part dividers (Part I / Part II), the trios-phd render binary, или просит изменить текст главы и пересобрать PDF. Anchor: φ²+φ⁻²=3."
license: MIT
metadata:
  author: trios-phd-pipeline
  version: '5.2'
  anchor: phi^2 + phi^-2 = 3
---

# phd-pipeline-v5 — Unified Flos Aureus + Neon Monograph Build

## When to Use This Skill

Load this skill when the operator asks for any of:

- "render PhD", "rebuild monograph", "обнови PhD", "сделай PDF диссертации"
- "v5.x", "v5.2", "v5.3" of the Flos Aureus + Neon book
- changes to a chapter (FA.NN, Ch.N, App.X, AP.X, FM.NN, Ch.0)
- "fix cover", "верни обложку", "cover unchanged"
- "part divider", "Part I / Part II", "spine", "page numbering"
- pandoc / tectonic failures inside the PhD render
- adding a new chapter into Neon `ssot.chapters`

This skill is the canonical owner of the v5.x render path.
It does **not** replace `phd-monograph-auditor` (defense lane) or `phd-chapter-author` (per-chapter author lane); it sits *between* them and produces the unified PDF.

## Constitutional Laws (R1, R5, R6)

* **R1 — Rust-only:** No `.sh`, no `.py`, no Makefile shell targets in the render path. All logic lives in `crates/trios-phd/src/render/mod.rs` and is invoked via `cargo run -p trios-phd --bin trios-phd -- render`.
* **R5 — Honest:** Every chapter that fails pandoc or tectonic gets a one-page placeholder (so the spine stays complete and page numbers stay continuous), and the failed slugs are reported in the as-flown table.
* **R6 — Lane isolation:** This skill does **not** edit `docs/phd/chapters/*.tex` (that is the `phd-chapter-author` lane) and does **not** edit `docs/phd/defense/*` (that is the `phd-monograph-auditor` lane). It only:
  - reads from Neon `ssot.chapters`,
  - writes to `crates/trios-phd/src/render/`,
  - emits `monograph.pdf` to `target/build/` (not committed).

## Anchor

\[\varphi^{2} + \varphi^{-2} = 3\]

Every commit message produced by this skill ends with `Anchor: phi^2 + phi^-2 = 3.`

---

## Architecture (As-Flown v5.2)

### Single Source of Truth: Neon `ssot.chapters`

```
ssot.chapters    (98 rows in v5.2)
├── Ch.0          (1)   manifesto — "Golden Ratio Parametrizations of SM Constants"
├── FM.01..FM.11  (11)  frontmatter (title, dedication, abstract, preface, …)
├── FA.00..FA.33  (34)  Flos Aureus chapters (Part I — sacred geometry + golden ratio)
├── AP.A..AP.H    (8)   Flos Aureus appendices
├── Ch.1..Ch.34   (34)  Neon computational chapters (Part II — TRINITY substrate)
└── App.A..App.J  (10)  Neon appendices
```

**Schema** (`ssot.chapters`):

| column | type | notes |
|---|---|---|
| `id` | int8 (PK) | autoincrement |
| `ch_num` | text UNIQUE NOT NULL | e.g. `FA.07`, `Ch.0`, `App.B` |
| `title` | text NOT NULL | human title |
| `status` | text NOT NULL | `drafted` / `reviewed` / `sealed` |
| `priority` | text | `core` / `phi-physics` / etc. |
| `evidence_axis` | int4 | nullable |
| `phi_seal` | bool | seal status |
| `word_count` | int4 | recomputed at upsert |
| `theorems_count` | int4 | rough regex count |
| `body_md` | text | **canonical content** (markdown + raw LaTeX) |
| `illustration_url` | text | hero image (raw GitHub URL) |
| `illustration_path` | text | repo-relative path |
| `updated_at` | timestamp | auto on upsert |

### Render Spine (As-Flown)

```
Cover (1pp)
  └─ Part I divider (1pp)
       └─ Ch.0 (manifesto)
            └─ FM.01..FM.11
                 └─ FA.00..FA.33
                      └─ AP.A..AP.H
                           └─ Part II divider (1pp)
                                └─ Ch.1..Ch.34
                                     └─ App.A..App.J
```

### Canonical Order Buckets

| bucket | range | use |
|--:|---|---|
| 0 | `Ch.0` | manifesto opens Part I |
| 1 | `FM.NN` | frontmatter |
| 2 | `FA.NN` | Flos Aureus core |
| 3 | `AP.X` | Flos appendices |
| 4 | `Ch.N` | Neon chapters |
| 5 | `App.X` | Neon appendices |
| 9 | unknown | sorts last |

Implementation: `canonical_order` + `part_divider_before` in `crates/trios-phd/src/render/mod.rs`.

---

## Workflow

### Step 0 — Pre-flight

```bash
# Verify the binary builds cleanly.
cargo build --release -p trios-phd --bin trios-phd
cargo test  -p trios-phd

# Verify Neon SoT.
psql "$NEON_URL" -c "SELECT count(*), sum(length(body_md)) FROM ssot.chapters"
# Expected (v5.2): 98 rows, ≈2.26 MB.
```

### Step 1 — Edit a chapter (Neon-first)

The operator's request will name a chapter (`FA.07`, `Ch.12`, etc.). Always edit the body in Neon — **never** in a workspace markdown file as the source of truth.

For body_md ≤25 KB, use the connector `neon_postgres-upsert-row` directly with the full row as `rowValues`.

For body_md >25 KB, use the **chunked-update CTE technique** (the only technique that bypasses the Pipedream 30-KB args limit). See `references/neon-write-techniques.md` for the full pattern.

Quick form:

```sql
-- 1. Insert/upsert metadata + empty body_md
INSERT INTO ssot.chapters (ch_num, title, status, priority, phi_seal, body_md, illustration_url, illustration_path)
VALUES ('FA.07', 'The Golden Sprout', 'drafted', 'core', false, '', '<url>', '<path>')
ON CONFLICT (ch_num) DO UPDATE
   SET title=EXCLUDED.title, body_md='', illustration_url=EXCLUDED.illustration_url,
       illustration_path=EXCLUDED.illustration_path, updated_at=NOW();

-- 2. Append body_md in 20-25 KB chunks (split at \n boundaries) via writeable CTE:
WITH upd AS (
  UPDATE ssot.chapters SET body_md = body_md || $1, updated_at=NOW()
  WHERE ch_num = $2 RETURNING ch_num, length(body_md) AS len
) SELECT * FROM upd;
-- $1 = next chunk text; $2 = ch_num. Use connector
-- `neon_postgres-find-row-custom-query` with values=[chunk, ch_num]
-- (find-row-custom-query routes data-modifying CTEs through, exec-custom-query rejects oversized args).
```

Verify after every upsert:

```sql
SELECT ch_num, length(body_md) FROM ssot.chapters WHERE ch_num = 'FA.07';
```

### Step 2 — Pull SoT into local cache

```bash
# Pull all rows into chapters.json (the renderer reads this when --neon-url is omitted).
# For ≤25 KB rows: SELECT ch_num, title, illustration_url, illustration_path, body_md
#                  FROM ssot.chapters WHERE ch_num IN (...)
# For >25 KB rows: substring(body_md, $1, 25000) loop until full body recovered.
# Re-assemble into [{ch_num, title, illustration_url, illustration_path, body_md}, ...]
# and write to /tmp/phd-render-out/chapters.json.

# Sanity-check:
python3 -c "import json; print(len(json.load(open('/tmp/phd-render-out/chapters.json'))))"
# Expected: 98 (or whatever total exists in ssot.chapters).
```

For the pull pattern, see `references/neon-read-techniques.md`.

### Step 3 — Render

```bash
export PATH=/home/user/workspace:$PATH         # for tectonic 0.16.9
cargo build --release -p trios-phd --bin trios-phd

cd /tmp/phd-render-out
~/path/to/target/release/trios-phd \
    --phd-root /tmp/phd-render-out \
    render --workdir /tmp/phd-render-out
```

Render pipeline (per chapter):

1. write `src/<ch>.md` from Neon body_md
2. preprocess (`preproc_body_md`): ZWSP after `/.?_=&` in inline code & URLs (margin overflow guard); strip control bytes; replace `$\sim$`/`$\approx$` with Unicode
3. pandoc → `<ch>.tex` (with Lua filter `force-fullwidth-hero.lua` that injects the hero image *after* `\chapter{...}`)
4. rewrite hero URL → relative `../assets/illustrations/<name>.png`
5. tectonic → `<ch>.pdf`

On failure (pandoc OR tectonic): the renderer writes a **placeholder one-page PDF** with the chapter slug and continues, so the spine never breaks. Failed slugs get logged with `⚠️`.

### Step 4 — Verify

```bash
qpdf --show-npages /tmp/phd-render-out/build/monograph.pdf
ls -la /tmp/phd-render-out/build/monograph.pdf
# v5.2 baseline: 422 pages, ≈30 MB.
```

Mandatory checks:

- Cover page renders unchanged (matches commit `8c3adb1`: 70-pt formula, 0.78\textwidth sunflower, `\null\vfill`-centred).
- Hero image appears **after** the chapter heading (not before), per `force-fullwidth-hero.lua` Lua filter.
- Page numbers continuous: cover=p.1, Part I=p.2, Ch.0=p.3, … no gaps.
- Failed-chapter placeholders have correct page numbers and slug labels.

### Step 5 — Commit & push

```bash
git add crates/trios-phd/src/render/mod.rs \
        crates/trios-phd/src/render/templates/*.tex \
        crates/trios-phd/src/render/filters/*.lua \
        docs/phd/skills/phd-pipeline-v5/

git commit -m "feat(phd render): vX.Y — <one-line summary>

* …
* …

R1-honest: pure Rust subcommand; no .py / .sh / Makefile invocations.
Anchor: phi^2 + phi^-2 = 3."

git push origin <branch>
```

PR comment template lives at `references/pr-comment-template.md`.

---

## Known Pitfalls (Anomaly → Corrective Action)

These are real failures observed during the v5.2 build. Apply the corrective action *before* re-rendering.

| Anomaly | Corrective action |
|---|---|
| `Unknown alias 'End'` from pandoc | Markdown source has `\n---\n*End` — pandoc reads it as YAML, where `*End` is an alias reference. Fix: replace `\n---\n*` with `\n***\n*` (use bullet HR before italic-leading text). |
| `Undefined control sequence \paragraphCorrespondence` | Hand-conversion glued the macro name to its argument. Fix in `body_md`: replace `\paragraphCorrespondence:` with `**Correspondence:**`. |
| `Missing $ inserted` in a longtable cell | A cell `$...$` got escaped because the cell content wraps onto two source lines OR contains a literal `\dots $` close that pandoc misreads. Fix: simplify the cell to a single short `$...$` expression, or move the long math into a display block above the table. |
| `Paragraph ended before \math@egroup was complete` | Bold markers `**...**` interleaved with `$...$` — e.g. `**$H_3\to E_8$**`. Fix: rewrite as `**$H_3 \to E_8$**` (single math span inside one bold) or `\textbf{$H_3 \to E_8$}`. |
| `! Undefined control sequence \theorem` | Template was missing `amsthm` declarations. Fixed in `chapter.template.tex` (v5.2): `\usepackage{amsthm}` + `\newtheorem{theorem}{Theorem}` + lemma/proposition/corollary/definition/remark/example. Do **not** revert. |
| `tectonic: missing illustration <name>.png` | The Neon `illustration_url` does not match a file on the `feat/illustrations` branch. Check `git ls-tree origin/feat/illustrations -- assets/illustrations/` for the canonical list and patch `chapters.json`. |
| `BEL` (0x07) byte in source | Some Flos chapters carry stray control bytes from hand-conversion. Always strip non-`\n\r\t` bytes <0x20 in `body_md` before pandoc. |
| Body unicode chars LaTeX can't render | Replacement table: `◻ → $\square$`, `✅ → [OK]`, `✓ → [check]`, `🔓 → [unlocked]`, `🚨 → [ALERT]`. Add new mappings as encountered. |
| Pipedream connector returns truncated body | Args >36 KB are silently truncated. Use the chunked-update CTE technique from `references/neon-write-techniques.md`. |
| Hero image appears *before* chapter heading | The Lua filter `force-fullwidth-hero.lua` inserts the image after the *first* `Header level=1`. If the body_md starts with the hero `![…](…)` *and then* `# Chapter Title`, the filter does the right thing. If hero appears before, check the Lua filter is being passed via `--lua-filter`. |

For new pitfalls discovered during v5.3+, append entries here and to `references/pitfalls.md`.

---

## Active Artefacts (v5.2)

| File | Owner | Purpose |
|---|---|---|
| `crates/trios-phd/src/main.rs` | this skill | CLI dispatch (`trios-phd render`) |
| `crates/trios-phd/src/render/mod.rs` | this skill | render orchestration, canonical order, part dividers, placeholder fallback |
| `crates/trios-phd/src/render/templates/cover.tex` | this skill | typeset cover (formula 70 pt, sunflower 0.78\textwidth) |
| `crates/trios-phd/src/render/templates/chapter.template.tex` | this skill | per-chapter pandoc template (DejaVu Serif + amsthm + xurl) |
| `crates/trios-phd/src/render/templates/part-divider.tex` | this skill | "Part I / II" page generator |
| `crates/trios-phd/src/render/filters/force-fullwidth-hero.lua` | this skill | inject hero image after `\chapter{…}` |
| `assets/illustrations/*.png` | feat/illustrations branch | hero images (45 PNGs in v5.2) |
| `ssot.chapters` (Neon) | the operator | canonical content (98 rows in v5.2) |

---

## What This Skill Does NOT Own

- `docs/phd/chapters/*.tex` — owned by `phd-chapter-author` lane
- `docs/phd/appendix/*.tex` — owned by `phd-chapter-author` lane
- `docs/phd/defense/*` — owned by `phd-monograph-auditor` lane
- Coq mechanisations — separate lane
- Reproducibility scripts — separate lane

If a request touches those, hand off to the appropriate lane (or pause and ask the operator).

---

## References

* `references/neon-write-techniques.md` — chunked CTE write pattern for >25 KB body_md.
* `references/neon-read-techniques.md` — substring-based pull for cache hydration.
* `references/pitfalls.md` — extended anomaly catalogue (deeper than this SKILL.md table).
* `references/pr-comment-template.md` — as-flown PR comment.

---

*Anchor: φ² + φ⁻² = 3.*
