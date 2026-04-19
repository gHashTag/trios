# [Title] — [Brief Description]

**Status:** [Planning/In Progress/Review/Complete]
**Date:** YYYY-MM-DD
**Authors:** [Author names]
**Issue:** #[Number]

---

## Executive Summary

[Brief overview of what this issue accomplishes and why it matters]

---

## Requirements

| Requirement | Priority | Status |
|-------------|-----------|--------|
| [Requirement 1] | P1 | [ ] |
| [Requirement 2] | P2 | [ ] |
| ... | | |

---

## Implementation Plan

| Phase | Description | Deliverable | Status |
|--------|-------------|-------------|--------|
| Phase 1 | [Description] | [Artifact] | [ ] |
| Phase 2 | [Description] | [Artifact] | [ ] |
| ... | | | |

---

## Technical Details

[Architecture, algorithms, specifications as needed]

---

## Testing Plan

| Test Type | Coverage | Expected Result |
|-----------|-----------|----------------|
| Unit | [Files/Modules] | [ ] |
| Integration | [Scenarios] | [ ] |
| Benchmark | [Metrics] | [ ] |

---

## Definition of Done (L0: Immutable)

**ALL** completed tasks MUST include these steps before merging:

```bash
# 1. Stage changes
git add <crate-path>/ <affected-files>

# 2. Commit with Issue reference
git commit -m "feat(<crate>): <description>

refs #<issue>
- <key changes bullet points>"

# 3. Push to remote
git push origin main
```

**Verification:**
- [ ] All crates modified are staged and committed
- [ ] Commit message contains `refs #<issue>`
- [ ] `git status` shows clean (no uncommitted changes)
- [ ] `cargo clippy -- -D warnings` passes (L3)
- [ ] `cargo test` passes (L4)

---

## References

- [Link 1]
- [Link 2]

---

## Next Actions

1. **[ ]** [Action 1]
2. **[ ]** [Action 2]

---

**Closes:** #<issue>
