# Extended pitfall catalogue (v5.0–v5.2)

Real failures observed during the v5.x build. Each entry: **symptom → root cause → corrective action**. Add new entries as discovered; keep them ordered roughly by frequency.

---

### P-001 · `Unknown alias 'End'` in pandoc

**Symptom**: pandoc exits with `Unknown alias 'End'` and no further context.

**Root cause**: pandoc reads the start of the body as a YAML metadata block. A horizontal rule `---` followed by `*End of body*` is parsed as YAML `*End` alias-reference because the leading `---` opened a YAML mapping.

**Fix**: replace `\n---\n*` with `\n***\n*` (use the bullet horizontal-rule form before italic-leading text). Apply to all chapters with this pattern.

---

### P-002 · `Undefined control sequence \paragraphCorrespondence`

**Symptom**: tectonic halts at a single line that ends with `\paragraphCorrespondence:`.

**Root cause**: regex-based hand conversion glued the macro name to its argument: `\paragraph{Correspondence}` was rewritten as `\paragraphCorrespondence`.

**Fix**: in `body_md`, replace `\paragraphCorrespondence:` with `**Correspondence:**` (markdown bold equivalent renders identically).

---

### P-003 · `Missing $ inserted` inside a longtable cell

**Symptom**: tectonic line-error in a cell whose content is supposed to be `$...$`.

**Root cause**: pandoc treats the cell as plain text (not math) in two cases:
- the cell content wraps onto two source lines,
- the cell ends with `\dots $` (pandoc heuristic).

**Fix**:
- Simplify the cell to a single short `$...$` expression.
- Move long math into a display block before the table, then reference it from the cell.

---

### P-004 · `Paragraph ended before \math@egroup was complete`

**Symptom**: tectonic halts deep inside an enumerated list with a math grouping error.

**Root cause**: bold markers `**...**` interleaved with `$...$` confused pandoc:
`**$H_3\to E_8\to\mathrm{SU**(3)$:}` — the second `**` closes bold mid-math.

**Fix**: rewrite as `**$H_3 \to E_8 \to \mathrm{SU}(3)$:**` (one `$...$` span inside one `**...**` group), or use `\textbf{$...$}`.

---

### P-005 · `! Undefined control sequence \theorem`

**Symptom**: tectonic fails on `\begin{theorem}` because the env is undefined.

**Root cause**: chapter template was missing `amsthm` declarations (v5.0–v5.1).

**Fix**: declared in v5.2 chapter template:
```latex
\usepackage{amsthm}
\newtheorem{theorem}{Theorem}
\newtheorem{lemma}[theorem]{Lemma}
\newtheorem{proposition}[theorem]{Proposition}
\newtheorem{corollary}[theorem]{Corollary}
\newtheorem{definition}[theorem]{Definition}
\newtheorem{remark}[theorem]{Remark}
\newtheorem{example}[theorem]{Example}
```
Do **not** revert this.

---

### P-006 · Mismatched display-math delimiters

**Symptom**: tectonic line-error inside what should be a display equation.

**Root cause**: hand-conversion produced bodies where `\[` opens but `$$` closes (or vice versa). LaTeX considers them different and refuses to balance them.

**Fix**: regex normalize before pandoc:
```python
body = re.sub(r'\\\[\s*([^[\]]*?)\$\$', r'\\[\n\1\n\\]', body, flags=re.DOTALL)
body = re.sub(r'\$\$\s*([^$]*?)\\\]',   r'\\[\n\1\n\\]', body, flags=re.DOTALL)
```

---

### P-007 · Missing illustration PNG

**Symptom**: `tectonic: cannot find file '../assets/illustrations/<name>.png'`.

**Root cause**: the `illustration_url` written into Neon does not match a file on `feat/illustrations`.

**Fix**:
```bash
git ls-tree origin/feat/illustrations -r --name-only -- assets/illustrations/
```
Pick the closest semantic match and patch:
```sql
UPDATE ssot.chapters
   SET illustration_url  = 'https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/<correct>.png',
       illustration_path = 'assets/illustrations/<correct>.png',
       updated_at = NOW()
WHERE  ch_num = '<ch_num>';
```

For new chapters, use the canonical mapping in `references/illustration-mapping.md` (TBD).

---

### P-008 · Stray control bytes in body_md

**Symptom**: tectonic halts with an unhelpful error referencing a line whose visible content is innocuous.

**Root cause**: hand-conversion left BEL (0x07) or other low control bytes in the markdown.

**Fix**: strip them before pandoc:
```python
body = ''.join(ch for ch in body if ord(ch) >= 0x20 or ch in '\n\r\t')
```
Apply to all chapters once; commit a fresh hydrate.

---

### P-009 · Unicode chars LaTeX cannot render

**Symptom**: tectonic line-error on a single character such as `◻` or an emoji.

**Root cause**: hand-conversion emitted Unicode primitives that the chosen font (DejaVu Serif) does not cover.

**Fix**: replacement table — apply during hydrate:

| char | replacement |
|---|---|
| `◻` (U+25FB) | `$\square$` |
| `✅` (U+2705) | `[OK]` |
| `✓` (U+2713) | `[check]` |
| `🔓` (U+1F513) | `[unlocked]` |
| `🚨` (U+1F6A8) | `[ALERT]` |

Add new entries as encountered.

---

### P-010 · Hero image appears before chapter heading

**Symptom**: PDF renders, but the hero illustration sits above `# Chapter Title` instead of below.

**Root cause**: pandoc was invoked without `--lua-filter=force-fullwidth-hero.lua`, so the body markdown's leading `![hero](url)` is treated as the first block.

**Fix**: confirm `run_pandoc` in `mod.rs` passes `--lua-filter` and that `templates/filters/force-fullwidth-hero.lua` is up to date. The filter inserts the hero `Image` *after* the first `Header level=1`.

---

### P-011 · Pipedream wrapper truncates body_md silently

**Symptom**: After upsert, `length(body_md)` returns ~28 KB instead of the expected size.

**Root cause**: `neon_postgres-upsert-row` and `neon_postgres-execute-custom-query` accept argument payloads only up to ~30 KB; they do not error on overflow.

**Fix**: use the chunked-CTE technique from `references/neon-write-techniques.md`. Always verify with a `length(body_md)` SELECT after every write.

---

### P-012 · Continuous page numbers break across part dividers

**Symptom**: PDF shows a jump from p.221 to p.223 after a Part divider.

**Root cause**: `next_page` counter in `mod.rs` was not incremented for the part divider's own page count.

**Fix**: in `mod.rs`'s render loop, after running tectonic on a part divider, do `next_page += pdf_page_count(&part_pdf)?` *before* moving to the next chapter. Do **not** use a hard-coded `+ 1`.

---

### P-013 · Cover regression

**Symptom**: cover renders with formula too small or sunflower offset.

**Root cause**: a commit changed `cover.tex` parameters away from the operator-agreed baseline (commit `8c3adb1`).

**Fix**: restore canonical params:
- formula `{\fontsize{70pt}{84pt}\selectfont \(\varphi^{2}+\varphi^{-2}=3\)\par}`
- sunflower `\includegraphics[width=0.78\textwidth]{cover_v4_top.png}`
- vertical centring via `\null\vfill ... \vfill`
- horizontal centring via `\begin{center} ... \end{center}`

The cover is a **sealed asset**. Do not modify without explicit operator approval.

---

*Anchor: φ² + φ⁻² = 3.*
