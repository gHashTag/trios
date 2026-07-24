# Skill: Zombie Issue Splitting

## When to use

When a GitHub issue contains multiple unrelated bugs (a "zombie" issue), or when an issue has been partially fixed but remains open because some sub-problems are unresolved. This blocks issue-count targets and obscures real remaining work.

## Why

- Multi-bug issues are impossible to close cleanly — one remaining bug keeps the whole issue open
- Atomic issues enable `Closes #N` traceability (L1 Law)
- Smaller issues are easier to assign, estimate, and verify

## Steps

1. **Identify zombie issues:**
   - Issue has >1 distinct symptom or root cause
   - Comments say "partially fixed" or "still broken in case X"
   - Issue open for weeks with no clear owner
   - Example: "Runtime bugs" covering cycle detection, cache key, StmtIf, StmtAssign — 4 separate bugs

2. **Audit each sub-bug:**
   - Can it be reproduced independently?
   - Does it have a unique root cause?
   - Can it be fixed in a single PR?
   - If yes to all → split it out

3. **Create atomic issues:**
   ```markdown
   ## Problem
   [Specific symptom in 1 sentence]

   ## Root cause
   [What code path causes it]

   ## Reproduction
   [Minimal steps or spec file]

   ## Acceptance criteria
   - [ ] Fix implemented
   - [ ] Regression test added
   - [ ] `cargo test` passes
   - [ ] Closes this issue

   ## Related
   - Original zombie issue: #X (partially fixed, split into this)
   ```

4. **Close the original zombie:**
   - Comment: "Split into atomic issues: #A, #B, #C. This issue is now a tracking issue; closing because the original scope is too broad."
   - Label: `zombie-split`

5. **Close atomic issues as fixed:**
   - Each atomic issue gets its own PR with `Closes #N`
   - Verify all sub-issues are closed before declaring the zombie resolved

## Example

**Before (zombie):**
- #968 — "Runtime bugs" (cycle detection + cache key + StmtIf + StmtAssign)

**After (atomic):**
- #1195 — "AST-driven run_asm ignores parsed input" → fixed
- #1196 — "run_sort discards sorted AST after sorting" → fixed
- #1197 — "convert_fn_to_comb drops control flow" → open
- #1198 — "@bitCast strict-aliasing UB" → open

## Anti-patterns

- **Don't split hairs:** If two symptoms share one root cause, keep them in one issue
- **Don't orphan sub-issues:** Always link back to the original zombie
- **Don't close without atomic replacements:** The work must still be tracked

## Related

- [[l1-l7-compliance]] — L1 TRACEABILITY requires `Closes #N`
- [[github-issue-analysis]] — Triage and audit workflow
