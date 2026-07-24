---
name: ci-fix
description: Fix failing CI checks for t27 PRs
---

# CI Fix for t27

Automatically fixes failing CI checks for t27 PRs.

## Checks Fixed

1. **Issue Gate** — Adds `Closes #N` to commits
2. **L1 TRACEABILITY** — Ensures `Closes #N` in commit messages
3. **NOW Sync Gate** — Updates `docs/NOW.md` with current date
4. **Coq Proofs** — Validates .v files (if changed)

## Usage

```
/ci-fix <issue_number>
```

## What It Does

1. Creates GitHub issue if needed
2. Updates `docs/NOW.md`:
   - Sets date to today (YYYY-MM-DD)
   - Adds entry for current work
3. Amends commit message with `Closes #N`
4. Pushes with `--force-with-lease`

## Example

```
/ci-fix 498
```

## Prerequisites

- Must be on feature branch
- Issue must exist in GitHub
- `docs/NOW.md` must be present
