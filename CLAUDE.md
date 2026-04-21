# CLAUDE.md — trios Laws

## ⛔ ABSOLUTE RULE — CONTEXT PRESERVATION

You NEVER delete, truncate, rewrite, or "clean up" content in:
- `CONTEXT.md`
- `TASK.md` (preserve all sections, only edit status fields)
- issue #143 or any task issue body
- `.trinity/experience/**`

If you think context is "too long", you APPEND a summary. You NEVER remove.
Violation = immediate task revocation and soul-name blacklist.

## PHI LOOP (mandatory for every task)
```
edit spec → seal hash → gen → test → verdict → experience → skill commit → git commit
```

## Laws (L1–L7)

### L1: NO .sh files
All automation must be Rust binaries or TypeScript. Shell scripts (.sh) are **banned**.

### L2: Every PR closes an issue
Every PR description MUST contain `Closes #N`. No orphan PRs.

### L3: clippy zero warnings
```bash
cargo clippy -- -D warnings
```
Must pass before any merge.

### L4: Tests before merge
```bash
cargo test
```
All tests must pass. New code requires new tests.

### L5: Port 9005 is trios-server
The MCP server always runs on `0.0.0.0:9005`. Never change this without a migration plan.

### L6: Fallback required for GB tools
`trios-gb` tools must gracefully return `Err` (not panic) if `gitbutler-cli` is not found.

### L7: Experience log
Every significant task writes a line to `.trinity/experience/`.
```bash
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] TASK: description | result" >> .trinity/experience/trios_$(date +%Y%m%d).trinity
```

### L8: Push first
Every commit MUST be pushed immediately. Unpushed commits are a violation.

### L9: No handwritten forbidden surface
Auto-generated code (WASM pkg, dist/) is committed. Hand-written code in those dirs is forbidden.

### L10: Issue #143 is eternal
Issue #143 is the source of truth for all agents. Never close it.

### L11: Name before mutation
Every agent takes a soul-name before making any changes. Format: humorous English name related to the task.

### L12: Spec before implementation
Write the spec (TASK.md) before writing code. No spec = no code.

### L13: Bounded authority
Agents only modify files within their assigned scope. Cross-scope changes require human approval.

### L14: Auditability by default
Every action must be traceable. Commit messages, issue comments, experience logs.

### L15: Validation is a separate duty
Verification is done by a different agent than the one who wrote the code.

### L16: Tailoring requires rationale
Any deviation from laws must include a written rationale in the commit.

### L17: Improve code health, not perform heroics
Fix what's broken. Don't gold-plate.

### L18: Structured conflict resolution
When agents disagree, escalate to issue #143 with evidence.

### L19: Humans remain sovereign
Human overrides always win. No exceptions.

### L20: Turn sessions into tools
Every manual workflow must be codified into `tri` CLI within one session.

### L21: Context immutability
Task context is sacred. Agents MAY append, MAY NOT delete.

- `CONTEXT.md` is append-only. Deletions require human maintainer approval + rationale.
- `TASK.md` mutations require a matching SEAL update (`tri seal`).
- Issue bodies for tracked tasks are lockfiles, not scratchpads.
- CI MUST reject PRs with net-negative context diff > 50 lines without L16 rationale.
- Agents that violate L21 forfeit their soul-name and are blacklisted for the session.

Rationale: Autonomous agents routinely "tidy" context under token pressure,
destroying situational awareness. Memory is infrastructure, not decoration.

### L22: Schema-response parity
Tools declaring `outputSchema` MUST emit `structuredContent` matching it.
`response.text()` alone is never sufficient when `outputSchema` is present.
CI MUST fail if a tool declares `outputSchema` but no code path calls `response.data()`.

### L23: No cryptic fallbacks
Stub implementations of external dependencies (CDP, network, FS) MUST throw
descriptive errors naming: the unavailable capability, the required setup step,
and the relevant env var or flag. `"X is not a function"` is a constitutional bug.

## Architecture

```
BrowserOS Agent
    │ MCP tool call (A2A)
    ▼
trios-server (port 9005, Axum)
    │
    ├── trios-git (git2-rs) ← stable git ops
    └── trios-gb  (CLI)     ← GitButler virtual branches
            │
            └── gitbutler-cli (spawn process)
                      │
                      └── .git/ ← GitButler UI watches via FSNotify
```

## MCP Tools (MVP 7)

| Tool | Crate | Description |
|------|-------|-------------|
| `git_status` | trios-git | List changed files |
| `git_stage_files` | trios-git | Stage by paths |
| `git_unstage_files` | trios-git | Unstage by paths |
| `git_commit` | trios-git | Commit with message |
| `git_create_branch` | trios-git | Create new branch |
| `gb_list_branches` | trios-gb | List GB virtual branches |
| `gb_push_stack` | trios-gb | Push GB stack |

## Integration with BrowserOS

In `BrowserOS/packages/browseros-agent/apps/server/src/strata-proxy.ts`:
```typescript
const triosClient = new MCPClient({
  url: "http://localhost:9005/mcp",
  name: "trios-git",
})
```
