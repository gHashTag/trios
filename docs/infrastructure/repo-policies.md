# Repo Policies — Trinity Hive

Anchor: `φ² + φ⁻² = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)

This document records cross-repo settings that are not captured in code but
are queen-ratified and must persist across operator hand-offs. Anything here
is a contract: if a setting changes, this file must change in the same PR.

## L-NOJS-FIX — companion workflow pattern (gHashTag/trios)

**Problem.** Branch-protection on `main` requires a check-run named
`no-js-check`. The original `.github/workflows/no-js.yml` only triggers when
paths under `crates/trios-ext/**` change. PRs that touch only docs /
assertions / proofs therefore never produce a `no-js-check` check-run, and
branch-protection sits in a "ghost required check" deadlock — mergeable but
blocked.

**Fix (queen-ratified, R3-clean — no admin bypass, no required-check
removal).** A companion workflow `.github/workflows/no-js-companion.yml`
mirrors the original paths-filter via `paths-ignore` and emits a job named
exactly `no-js-check` that always exits 0. The two workflows are mutually
exclusive by construction:

| PR scope | `no-js.yml` | `no-js-companion.yml` |
| --- | --- | --- |
| changes under `crates/trios-ext/**` | runs (real lint) | skipped |
| changes anywhere else | skipped | runs (auto-pass) |

Same context name → branch-protection always satisfied. Real handwritten-JS
lint coverage on `crates/trios-ext/**` is unchanged.

**Audit trail.** Original deadlock observed on
[trios#541](https://github.com/gHashTag/trios/pull/541) (G4 — matrix-ledger
appendix, docs-only). Queen verdict locked the sequence
L-NOJS-FIX → L-COQ47 → L-EMBED-CRON.

## trios-trainer-igla — workflow permissions (queen-ratified)

The `format-algo-matrix.yml` workflow auto-opens a PR with refreshed
`assertions/matrix_samples.jsonl`. To do so the bot needs to create PRs and
the operator must avoid manual rebasing each cycle. The following
repository-level settings are therefore set on
[gHashTag/trios-trainer-igla](https://github.com/gHashTag/trios-trainer-igla):

```
default_workflow_permissions:        write
can_approve_pull_request_reviews:    true
```

These were enabled with explicit operator confirm_action approval during
L-MATRIX-LEDGER (G3 close, see PR #109 squash `d6f13e1`). They are scoped to
trios-trainer-igla only — `gHashTag/trios` itself keeps the GitHub default
(`read`).

**Why this is safe.** Branch-protection on trios-trainer-igla `main` requires
the `ci` context, `strict=true`, `required_linear_history=true`, and
`enforce_admins=true`. Auto-merge is DISABLED at the repo level. Therefore a
bot-opened PR cannot land without (a) the data-content `ci` job passing on
the bot's branch and (b) a human (or another bot with review rights)
approving the merge.

**Bot identity.** The matrix-ledger bot commits as
`trios-matrix-ledger-bot <matrix-ledger-bot@users.noreply.github.com>` to
branches matching `auto/matrix-ledger-${GITHUB_RUN_ID}`. PR title pattern:
`chore(matrix-ledger): refresh assertions/matrix_samples.jsonl (run ${GITHUB_RUN_ID})`.

## Constitutional Enforcement (L2) — PR-body regex

Branch-protection on `gHashTag/trios` `main` requires the `Constitutional
Enforcement` context. The Laws Guard scans the PR body for:

```
(Closes|Fixes|Resolves) #[0-9]+
```

(case-insensitive). Every PR — including L-NOJS-FIX and any docs-only
follow-up — must include such a clause referencing the tracking issue.

## R3 PR-only rule (no force-push, no admin merge)

`enforce_admins=true` on both `trios/main` and `trios-trainer-igla/main`. Any
"Bypass branch-protection" intervention is a R3 violation and is forbidden
without an explicit queen verdict recorded in this document. Stale PR
branches must be brought up to date via `gh pr update-branch <N>` (creates a
merge commit from main into the PR branch, R3-clean) — never via local
force-push.

## Zenodo DOI registry

All Zenodo DOIs authored by Dmitrii Vasilev are catalogued in [`docs/infrastructure/zenodo-registry.md`](./zenodo-registry.md). When citing a DOI in `info.yaml`, README, LICENSE, ADR, paper, or commit message, verify the title against that registry. Do not invent sub-titles that are not present in Zenodo metadata.
## Authorship — Dmitrii Vasilev (standing rule)

The author of all Trinity / gHashTag / trios / trinity-clara / trinity-fpga
artefacts is **Dmitrii Vasilev** (`raoffonom@icloud.com`). This includes:

- `SKILL.md` `metadata.author`
- `info.yaml` / TinyTapeout G6 submission `author`
- PhD monograph title page (`\author{}`) and arXiv authorship
- Zenodo `creators`, Apache LICENSE copyright holder
- ADR sign-offs, README headers
- Git commit `Author` and `Co-authored-by` trailers

The handle `gHashTag` is **the GitHub organization namespace only**. It is
NOT a person and must NEVER be used as `author`, `creator`, or `by`
attribution. Legitimate uses of `gHashTag`:

- Inside repo URLs (`github.com/gHashTag/<repo>`)
- As organization name in CI log context
- As the literal skill name of `phd-chapter-author` (skill names are not
  attribution)

Any other occurrence is an R5 violation and must be corrected on sight.

### Historical audit (prior author-leak incidents)

Recording here so the audit trail is immutable:

- [trios#544](https://github.com/gHashTag/trios/pull/544) (L-NOJS-FIX,
  squash `ae4e7ff` on `main`): mis-authored as
  `trios-matrix-ledger-bot <matrix-ledger-bot@users.noreply.github.com>`
  due to leftover git config from the preceding L-MATRIX-LEDGER
  trainer-igla auto-PR session. True author: **Dmitrii Vasilev**. Cannot
  be rewritten on `main` (R3 PR-only, no force-push); this entry is the
  canonical correction.
- [trios#546](https://github.com/gHashTag/trios/pull/546) (L-COQ47 INV-7,
  commit `3a0eaf9`): same mis-author. Corrected on the feature branch by
  add-commit `69cfe29` carrying `Co-authored-by: Dmitrii Vasilev`, so the
  squash-merge will attribute correctly.

### Operator hygiene

After each trainer-igla bot session, the local `git config user.name` /
`user.email` must be reset to `Dmitrii Vasilev` / `raoffonom@icloud.com`
before any hand-written commit. The matrix-ledger bot identity is
reserved exclusively for the `format-algo-matrix.yml` auto-PR flow on
`gHashTag/trios-trainer-igla`.
