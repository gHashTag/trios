# Skill: GitHub CLI Auth Workaround

## When to use

When `gh` commands fail with `HTTP 401: Bad credentials` because `GH_TOKEN` is invalid or expired, but a valid account exists in the macOS keyring.

## Why

- `GH_TOKEN` env var overrides keyring-stored credentials
- If `GH_TOKEN` is invalid, ALL `gh` commands fail even when keyring has valid token
- Common in long-running sessions where env var was set but token expired

## Diagnosis

```bash
gh auth status
# Output:
#   X Failed to log in to github.com using token (GH_TOKEN)
#   - Active account: true
#   ✓ Logged in to github.com account gHashTag (keyring)
```

If you see `X Failed ... GH_TOKEN` + `✓ Logged in ... keyring`, the workaround applies.

## Workaround

Prefix ALL `gh` commands with `env -u GH_TOKEN`:

```bash
# Instead of:
gh issue list --state open

# Use:
env -u GH_TOKEN gh issue list --state open

# Close an issue:
env -u GH_TOKEN gh issue close 123 --comment "Fixed in PR #456. Closes #123."

# Create an issue:
env -u GH_TOKEN gh issue create --title "Bug" --body "..."
```

## Permanent Fix

1. **Option A: Regenerate token**
   ```bash
   gh auth refresh --scopes repo,read:org,gist,admin:public_key
   export GH_TOKEN=$(gh auth token)
   ```

2. **Option B: Remove GH_TOKEN from shell config**
   ```bash
   # In ~/.zshrc, ~/.bashrc, or ~/.zshenv, remove or comment out:
   # export GH_TOKEN=ghp_...
   ```
   Then restart shell and rely on keyring auth.

3. **Option C: Use keyring in scripts**
   ```bash
   # In CI scripts or aliases:
   alias gh='env -u GH_TOKEN gh'
   ```

## Verification

```bash
env -u GH_TOKEN gh issue view 1 --json number,title
# Should succeed and return issue data
```

## Related

- [[zombie-issue-splitting]] — Cleaning up old multi-bug issues
- [[l1-l7-compliance]] — L1 TRACEABILITY requires `Closes #N`
