# Defense Rehearsal Log

> ≥3 rehearsals required before viva. Each entry must include date, length,
> self-critique against R5/R7/R11/R12, and any pivots committed back to chapters.
>
> **R5 honest discipline:** entries below 2026-06-15 (defense window opens) are
> placeholder rows. The auditor lane (`phd-monograph-auditor` v1.0) will not
> back-fill them silently; rehearsal coordinators sign each row with their
> agent handle and commit SHA on the rehearsal day.

| # | Date (UTC) | Duration | Coordinator | Self-critique notes (R5/R7/R11/R12) | Pivots committed |
|---|------------|----------|-------------|-------------------------------------|------------------|
| 1 | TBD (≤ 2026-05-15) | TBD (≤ 90 min) | TBD | TBD | TBD |
| 2 | TBD (≤ 2026-05-30) | TBD (≤ 90 min) | TBD | TBD | TBD |
| 3 | TBD (≤ 2026-06-10) | TBD (≤ 90 min) | TBD | TBD | TBD |

## Critique rubric (R-rule alignment)

- **R5** — every `Admitted` is named explicitly; no silent flips. Two
  `Admitted` entries (`igla_found_corroboration`,
  `nca_band_corroboration`) must be disclosed on Slides 5 and 11/13.
- **R7** — every empirical claim has a falsifier visible on the matching
  slide. The witness command must be reproducible in one line of cargo.
- **R11** — every cited work is in `bibliography.bib`. No
  unprintable-PDF citations.
- **R12** — pivot any blocker > 30 min on a single slide; never silently
  drop the slide.

## Rehearsal scheduling intent

- Rehearsal 1 (≤ T-30d): full 30-slide walkthrough; focus is timing and
  R5 disclosure.
- Rehearsal 2 (≤ T-15d): targeted Q&A drill on the 30-pair anticipated
  question set (`anticipated-questions.tex`); focus is brevity and
  fielding adversarial probing.
- Rehearsal 3 (≤ T-5d): full dress rehearsal; the `defense_gate` witness
  binary is run once at the start (must exit zero) and once at the end
  (must still exit zero).

## Auditor stamp

This log was extended (skeleton → 3-row table + rubric + scheduling
intent) by the LD lane body fill. Substantive rehearsal entries are
filled in by the rehearsal coordinator on the rehearsal day; entries are
committed atomically per R10 (`feat(phd-LD): rehearsal N entry [agent=<id>]`).

Skeleton seeded by `phd-monograph-auditor` v1.0 cycle 2 (commit
`60d87cf`). Body fill by `phd-monograph-auditor` v1.0 cycle 3 (LD lane
DONE). Rehearsal entries: pending coordinator claim per R9.
