# NOW Update for t27

Updates `docs/NOW.md` with current date and work entry.

## Usage

```
/now-update "<work-title>"
```

## What It Does

1. Reads current `docs/NOW.md`
2. Updates `Last updated` line to today's date (YYYY-MM-DD)
3. Adds new work entry:
   ```markdown
   ## YYYY-MM-DD — <work-title>
   - Description bullet points
   - Additional details
   ```
4. Stages file for commit

## Example

```
/now-update "Ring 018 — compiler cleanup"
```

Adds:
```markdown
## 2026-04-18 — Ring 018 — compiler cleanup
- Restored compiler/ modules
- Removed spurious lib.rs
- Fixed CI checks
- PR #497, Closes #498
```

## Prerequisites

- `docs/NOW.md` exists
- Working directory clean (or only NOW.md staged)
