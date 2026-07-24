# Skill: Batch Issue Closure

## When to use

When you need to close multiple stale, obsolete, or deferred issues in a single wave loop. This is common after audit sweeps reveal long-open issues with no progress.

## Why

- Keeping stale issues open inflates the open-issue count and obscures real work
- Batch closure with honest justification notes maintains L1 TRACEABILITY
- Closing superseded issues signals project hygiene and focus

## Steps

1. **Identify closure candidates:**
   - Issues >3 weeks old with no assignees
   - Issues superseded by newer work or roadmap
   - Issues that are "enhancements" with no linked PRs
   - Partially-fixed zombie issues (after splitting remaining work into atomic issues)

2. **Prepare honest justification for each:**
   - Why is it being closed? (stale, superseded, deferred, completed)
   - What replaced it? (newer issue, different approach, no longer relevant)
   - Is the work truly done or just deferred?

3. **Use GitHub CLI with auth workaround:**
   ```bash
   # If GH_TOKEN is invalid, use keyring fallback:
   env -u GH_TOKEN gh issue close <N> --comment "Wave Loop X audit: ..."
   ```

4. **Verify closure:**
   ```bash
   env -u GH_TOKEN gh issue list --state open --limit 1 | wc -l
   # Should show reduced count
   ```

5. **Document in report:**
   - List all closed issues with reason
   - Net effect on open issue count
   - Any blockers or failures

## Common Patterns

| Pattern | Justification Template |
|---------|----------------------|
| Superseded | "Superseded by newer work / different architecture. Closing." |
| Stale | "No activity for N weeks, no assignees, no linked PRs. Closing as stale." |
| Deferred | "Not on current critical path. Closing as deferred; will reopen when scheduled." |
| Completed | "Work completed in other issues / commits. Closing as completed." |
| Zombie split | "Split into atomic issues: #A, #B. Closing parent as tracking issue." |

## Anti-patterns

- **Don't close without honest note** — every closure must explain why
- **Don't close CRITICAL bugs** — only close tracking issues, enhancements, or stale features
- **Don't fabricate closures** — if auth fails, document the failure honestly
- **Don't create new zombies** — if work remains, create atomic issues before closing parent

## Related

- [[zombie-issue-splitting-v2]] — Splitting multi-bug issues before closure
- [[gh-cli-auth-workaround]] — When GitHub CLI auth is broken
- [[l1-l7-compliance]] — L1 TRACEABILITY requires `Closes #N`
