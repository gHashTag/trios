# Defense Rehearsal Log

> ≥3 rehearsals required before viva. Each entry must include date, length,
> self-critique against R5/R7/R11, and any pivots committed back to chapters.

Defense window: **2026-06-15** (UTC). Author-driven rehearsal events — R5 forbids
fabricating completion entries. The skeleton below is the schedule plan; rows are
filled in by the rehearser themselves after each session.

| # | Planned date (UTC) | Duration | Self-critique notes | Action items |
|---|--------------------|----------|---------------------|--------------|
| 1 | 2026-05-25 ± 3 d | 90 min target | _pending_ | _pending_ |
| 2 | 2026-06-05 ± 3 d | 90 min target | _pending_ | _pending_ |
| 3 | 2026-06-12 ± 1 d | 60 min target | _pending_ | _pending_ |

**Scheduling rationale (R5-honest):**

- Rehearsal 1 (T-21d): full 90-min walkthrough of all 30 slides + Q&A live drill.
  Focus: catch any forbidden-seed slip, verify every Admitted is named on its slide.
- Rehearsal 2 (T-10d): 90-min adversarial run with examiner-pack-style questioning.
  Focus: numerical anchors trace via `\citetheorem` to appendix F.
- Rehearsal 3 (T-3d): 60-min final timing drill. No content edits after this point
  except critical fact corrections.

**Reminder cron (suggested):** `0 9 25 5 *` UTC for rehearsal 1 reminder ping.
**Pre-rehearsal checklist:** ACM AE pack reachable, Coq map (App.~F) regenerated
within the last 7 days, bibliography fresh-pull from `bibliography.bib` HEAD.

## Critique rubric (R-rule alignment)

When filling each row, the rehearser MUST score themselves on:

- **R5** every Admitted is named explicitly on a slide; no silent flips.
- **R7** every empirical claim has a falsifier on the corresponding slide.
- **R11** every cited work is in `bibliography.bib`.
- **R12** all in-slide cites use numeric `[n]` brackets.
- **R14** every numeric anchor traces via `\citetheorem{INV-k}` to appendix F.
- **R12 (lane)** any blocker on a single slide must be self-pivoted within 30 min.

## Auditor stamp

Skeleton seeded by `phd-monograph-auditor` v1.0 cycle 2.
Body filled by same skill in cycle 4 (Phase C of ONE SHOT v2.0).
Rehearsal entries are author-claimable via R9.
