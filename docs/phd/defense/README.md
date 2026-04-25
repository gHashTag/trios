# Flos Aureus PhD — Defense Package (LD lane)

> Auditor-seeded skeleton (R6 boundary: structure only, no chapter content).
> Per `phd-monograph-auditor` v1.0 SKILL.md Step 8, this lane is the **last gate** —
> it depends on all 33 chapters DONE + appendices B/F/G/H green.

## Directory layout

```
defense/
├── README.md                    — this file
├── slides/                      — 30 Beamer slides (5 intro + 20 chapter + 5 Q&A)
│   └── _outline.md              — slide-by-slide outline (auditor seed)
├── examiner-pack.tex            — 50-page external examiner summary (author lane)
├── rehearsal-log.md             — log of ≥3 rehearsals with self-critique
├── anticipated-questions.tex    — 30 Q&A pairs (≤200 words each)
└── public-summary.md            — 1-page popular summary (CC-BY-4.0)
```

## Lane ownership

- **Skeleton (this PR)** — `phd-monograph-auditor` (auditor lane LD).
- **Substantive content** — `phd-chapter-author` agents claim individual files via R9 claim-before-work.
- **Cross-cutting QA** — `phd-monograph-auditor` cycles 4+ run pagecount + presence checks once content exists.

## R-rule trace

- **R6** auditor seeds only this directory + `_outline.md`; chapter authors fill in the substantive `.tex`/`.md` files.
- **R8** examiner-pack ≤ 50 pages; slides exactly 30; anticipated-questions ≤ 30 pairs.
- **R12** lane runs LAST; do not begin until B/F/G/H are 🟢 PASS.
