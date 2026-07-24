# PR Create for t27

Creates a PR that passes all CI checks.

## Usage

```
/pr-create <branch-name> <issue-number> "<title>"
```

## What It Does

1. Creates feature branch from current HEAD
2. Updates `docs/NOW.md`:
   - Sets date to today (YYYY-MM-DD)
   - Adds entry for current work
3. Amends top commit with `Closes #N`
4. Pushes branch to origin
5. Outputs PR creation URL

## Example

```
/pr-create fix/ring-018-compiler-cleanup 498 "fix: restore compiler modules"
```

## Output

```
Branch: fix/ring-018-compiler-cleanup
Pushed to origin
Create PR: https://github.com/gHashTag/t27/pull/new/fix/ring-018-compiler-cleanup
```

## Prerequisites

- Clean working directory (or staged changes only)
- Issue exists in GitHub
- `docs/NOW.md` exists
