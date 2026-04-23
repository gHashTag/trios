# CLAUDE.md — trios Laws

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

### L8: PUSH FIRST LAW
Every file change = immediate commit + push. There is no such thing as "done locally".

Checklist before saying "done":
```
git status — 0 untracked/modified files
git log --oneline -3 — commit is visible
github.com/gHashTag/trios — file visible in browser
```

If a file is not in the repo — the task is NOT complete.

## Agent Dispatch

To dispatch an agent to any GitHub issue, use the ONE-SHOT prompt:

```
.trinity/prompts/agent-dispatch.md
```

Replace `{{ISSUE_NUMBER}}`, `{{ISSUE_TITLE}}` — the agent picks its own soul-name.
The prompt embeds all LAWS (L1–L9), the full PHI LOOP (11 steps), HEARTBEAT format,
architecture overview, and a DONE checklist that blocks premature victory declaration.

See: [.trinity/prompts/agent-dispatch.md](.trinity/prompts/agent-dispatch.md)

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
