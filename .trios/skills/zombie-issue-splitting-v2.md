# Skill: Zombie Issue Splitting v2 (Post-W89 Refinement)

## When to use

When a GitHub issue contains multiple unrelated bugs (a "zombie"), or when an issue has been partially fixed but remains open because some sub-problems are unresolved. This blocks issue-count targets and obscures real remaining work.

## Why

- Multi-bug issues are impossible to close cleanly — one remaining bug keeps the whole issue open
- Atomic issues enable `Closes #N` traceability (L1 Law)
- Smaller issues are easier to assign, estimate, and verify
- **Post-W89 insight:** Splitting zombies may temporarily increase issue count (52 → 55 in W89), but the *quality* of the backlog improves dramatically — every issue becomes closable

## Steps

1. **Identify zombie issues:**
   - Issue has >1 distinct symptom or root cause
   - Comments say "partially fixed" or "still broken in case X"
   - Issue open for weeks with no clear owner
   - CRITICAL label with 7 bugs in one title (#971 W89 example)

2. **Audit each sub-bug:**
   - Can it be reproduced independently?
   - Does it have a unique root cause?
   - Can it be fixed in a single PR?
   - If yes → split it out

3. **Create atomic issues:**
   ```markdown
   ## Problem
   [Specific symptom in 1 sentence]

   ## Root cause
   [What code path causes it]

   ## Acceptance criteria
   - [ ] Fix implemented
   - [ ] Regression test added
   - [ ] `cargo test` passes
   - [ ] Closes this issue

   ## Related
   - Original zombie issue: #X (split into this)
   ```

4. **Comment on parent zombie:**
   - Explain split: "Split into atomic issues: #A, #B, #C"
   - Link new issues

5. **Close the parent zombie:**
   - Comment: "Closing parent zombie. All remaining work tracked in atomic issues above."
   - Label: `zombie-split`

6. **Accept temporary count increase:**
   - 1 zombie with 7 bugs → 4 atomic issues = +3 net issues
   - This is CORRECT — the backlog is now honest and trackable
   - Do NOT avoid splitting to keep count low — that creates zombie debt

## Example (W89)

**Before (zombie):**
- #971 — "W95 R-COMPILER: VCD truncation, testbench timeout, seal SHA, parser DotDot, BitstreamMeta, parse_type_annotation, ..." (7 bugs, CRITICAL, zero assignees)

**After (atomic):**
- #1199 — VCD truncation >32 bits
- #1200 — Testbench timeout race
- #1201 — Seal SHA hex length
- #1202 — Parser DotDot precedence
- #971 — **CLOSED** as zombie split

**Net effect:** Issues count increased by +3, but each issue is now assignable, trackable, and closable.

## Anti-patterns

- **Don't avoid splitting to keep issue count low** — zombie debt is worse than honest count
- **Don't split hairs** — if two symptoms share one root cause, keep them in one issue
- **Don't orphan sub-issues** — always link back to the original zombie
- **Don't close without atomic replacements** — the work must still be tracked

## Related

- [[l1-l7-compliance]] — L1 TRACEABILITY requires `Closes #N`
- [[github-issue-analysis]] — Triage and audit workflow
- [[gh-cli-auth-workaround]] — Running `gh issue create` when auth is broken
