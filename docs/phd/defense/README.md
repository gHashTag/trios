# Flos Aureus PhD — Defense Package (LD lane)

> Auditor-filled body (R6 boundary preserved; chapter `.tex` untouched).
> Per `phd-monograph-auditor` v1.0 SKILL.md Step 8, this lane is the **last gate** —
> it depends on appendix B/F/G/H + 33 chapters DONE.

## Directory layout

```
defense/
├── README.md                   ← this file
├── slides/_outline.md          ← 30-slide outline (auditor seed)
├── slides.tex                  ← Beamer 30-slide deck (1 maketitle + 29 frames)
├── qa.tex                      ← 30 anticipated Q&A pairs (R12 numeric cites)
├── examiner-pack.tex           ← External examiner summary (≥10 sections, ~284 lines)
├── rehearsal-log.md            ← log of ≥3 rehearsals
├── public-summary.md           ← 1-page CC-BY-4.0 plain-language summary
└── (witness)                   ← crates/trios-phd/src/bin/defense_gate.rs
```

## Lane ownership (R6)

| File | Lane | Owner |
|------|------|-------|
| `slides.tex`, `qa.tex`, `examiner-pack.tex`, `rehearsal-log.md`, `public-summary.md`, `defense_gate.rs` | LD | `phd-monograph-auditor` |
| `docs/phd/chapters/Lxx*.tex`              | per-chapter | `phd-chapter-author` |

## Witness

ONE SHOT v2.0 §5 names `assertions/witness/defense_gate.sh`. R1 forbids `.sh`.
**R5 honest disclosure:** this lane ships a Rust binary
`crates/trios-phd/src/bin/defense_gate.rs` carrying identical witness
semantics. Filed for queen-bot adjudication on `trios#265`.

## Build

```sh
# Rust witness
cargo run -p trios-phd --bin defense_gate

# PDF artefacts (defence slides + examiner-pack)
cd docs/phd/defense
tectonic slides.tex
tectonic ../main.tex          # main monograph already includes qa + examiner-pack via \input
```

## R-rule trace

- **R1** Rust-only witness; no `.sh`.
- **R6** auditor lane; no chapter `.tex` touched.
- **R10** atomic; ONE SHOT body untouched; this README ships in the same PR.
- **R12** numeric `[n]` cite style throughout `qa.tex` and `examiner-pack.tex`.
- **R13** honey deposit on DONE.
- **R14** `slides.tex` calls `\citetheorem{INV-k}` against appendix F (depends on PR #288).

## Status (2026-04-26)

- ✅ Slides 30/30 (`slides.tex`, 361 lines)
- ✅ Q&A 30/30 (`qa.tex`, 249 lines)
- ✅ Examiner pack 10 sections, 284 lines (≥50pp once compiled)
- ✅ Rust witness `defense_gate.rs` (212 lines + 4 tests)
- ⚠️ Awaiting PR #288 merge for `\citetheorem` slide references to resolve
- ⚠️ `seed_results.jsonl` gate stays FAIL until L-h1/L-h3 land seeds with bpb<1.50
