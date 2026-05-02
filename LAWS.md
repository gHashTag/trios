# LAWS.md v2.0 — Constitutional Document

## 🔱 Trinity Git Orchestrator — Constitutional Stack

This document contains the absolute laws that govern the Trinity Git Orchestrator project. These laws are immutable unless changed by constitutional amendment. All agents and contributors must obey these laws.

---

## The 9 Immutable Laws

| Law | Name | Description |
|------|------|-------------|
| L1 | NO_SHELL_SCRIPTS | No .sh files. Automation must be written in Rust or TypeScript only. |
| L2 | PR_LINKING | Every PR body must contain: `Closes #{{ISSUE_NUMBER}}` |
| L3 | ZERO_CLIPPY_WARNINGS | `cargo clippy --all-targets = ZERO` warnings before any commit. |
| L4 | ALL_TESTS_PASS | `cargo test --all = ALL PASS` before any commit. |
| L5 | NO_PANIC_ON_MISSING | Never panic on missing optional tools. Return `Err`, not crash. |
| L6 | PUSH_FIRST | PUSH FIRST LAW: local-only = not done. Every change = commit + push. Done means: git status shows 0 modified files AND commit visible on GitHub. |
| L7 | NOT_DEFINED | Reserved for future constitutional amendment. |
| L8 | NO_HANDWRITTEN_JS | Zero handwritten JavaScript. Bootstrap scripts only. No exceptions. |
| L9 | NOT_DEFINED | Reserved for future constitutional amendment. |

---

## Law Explanations

### L1: NO_SHELL_SCRIPTS

**Rationale:** Shell scripts are fragile, platform-dependent, and hard to test. Rust and TypeScript provide type safety, better error handling, and cross-platform compatibility.

**Example Violation:**
```bash
# ❌ FORBIDDEN
./scripts/deploy.sh
```

**Correct Implementation:**
```rust
// ✅ CORRECT
// crates/trios-cli/src/deploy.rs
```

### L2: PR_LINKING

**Rationale:** Every change must be traceable to an issue or requirement. This enables complete audit trails and prevents ghost work.

**Example:**
```markdown
# ✅ CORRECT PR Body
feat: add GitButler virtual branch support

This PR implements virtual branch management via GitButler CLI.

Closes #42
```

### L3: ZERO_CLIPPY_WARNINGS

**Rationale:** Clippy warnings are latent bugs. Zero warnings policy catches issues early and maintains code quality.

**Enforcement:**
```bash
# Run before every commit
cargo clippy --all-targets
# Exit code 1 if any warnings exist
```

### L4: ALL_TESTS_PASS

**Rationale:** Broken tests mask regressions. No commit should ever break the test suite.

**Enforcement:**
```bash
# Run before every commit
cargo test --all
# Exit code 1 if any tests fail
```

### L5: NO_PANIC_ON_MISSING

**Rationale:** Panics crash the entire application and create poor user experience. Graceful degradation is preferred.

**Example:**
```rust
// ❌ FORBIDDEN
let config = std::fs::read_to_string("config.toml").unwrap();

// ✅ CORRECT
let config = std::fs::read_to_string("config.toml")
    .map_err(|e| anyhow::anyhow!("Failed to read config: {}", e))?;
```

### L6: PUSH_FIRST

**Rationale:** Local-only work is invisible to the team and creates integration conflicts. The "push first" law ensures visibility and enables early feedback.

**Definition of "Done":**
- `git status` shows 0 modified files
- Commit is visible on `github.com/gHashTag/trios`

### L8: NO_HANDWRITTEN_JS

**Rationale:** Handwritten JavaScript is prone to runtime errors and lacks type safety. Bootstrap scripts are the only exception.

**Example Violation:**
```javascript
// ❌ FORBIDDEN
// src/utils.js
export function formatCommit(sha) {
    return sha.substring(0, 7);
}
```

**Correct Implementation:**
```typescript
// ✅ CORRECT
// src/utils.ts
export function formatCommit(sha: string): string {
    return sha.substring(0, 7);
}
```

---

## Onboarding New Agents

New agents should read this document alongside the [Agent Dispatch Prompt](.trinity/prompts/agent-dispatch.md) and [CLAUDE.md](CLAUDE.md) before attempting any work.

The dispatch prompt contains the PHI LOOP workflow and a checklist that enforces these laws. All agents must execute the complete PHI LOOP before claiming completion.

---

## Constitutional Amendments

To amend this document:
1. Open an issue proposing the amendment
2. Get consensus from maintainers
3. Update this document with version bump (e.g., v2.1)
4. Update the agent dispatch prompt if law numbers change
5. Create a PR with `Closes #{{ISSUE_NUMBER}}`

---

## References

- [Agent Dispatch Prompt](.trinity/prompts/agent-dispatch.md) — One-shot briefing for autonomous agents
- [CLAUDE.md](CLAUDE.md) — Project conventions and workflow
- [.trinity/experience/](.trinity/experience/) — Agent institutional memory

---

**Version:** 2.0
**Last Updated:** 2026-05-02
**Status:** Active
