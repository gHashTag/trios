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

## Own language first

When this project publishes something about itself, it publishes in **this
project's own language and format** -- not translated into somebody else's.

Owner's rule, 2026-09-20: stop writing in other people's languages, we have our
own.

This bites on any file whose only reason to exist is that an outside tool
expects that shape: `llms.txt`, `agents.json`, `ai.txt`, `.well-known/*.json`,
A2A agent cards, `ai-plugin` manifests, OpenAPI stubs, JSON-LD blocks, a README
that restates a spec. The reflex is to write four of them in four foreign
formats, and the reflex is wrong: a project whose claim is "here is a language
worth writing" and which then describes itself in three of other people's
formats has published three documents that are not true of it.

**The move:** find the address the outside world already fetches, then serve our
own language at it. `/llms.txt` at t27.ai **is** a t27 module -- `llms.txt`
requires nothing but text, and every prose line of a `.t27` file is a `;`
comment, so it stays readable to anything that cannot compile it.

**Three qualifications, so the rule stays honest:**

- A format a resolver genuinely parses -- a sitemap, `package.json`, a lockfile
  -- is machinery, not a description. **Generate** it from our own source; never
  hand-write it into a second home for the truth.
- Code against someone else's API uses their types. Prose for a human who has
  never heard of the project uses that human's language.
- If a format demands a claim we cannot back, **publish nothing**. An A2A card
  with no A2A server behind it is a false claim, and a missing file is more
  honest than a lying one.

The test: *is this file the project speaking about itself?* If yes, it speaks
our language. If it is plumbing, it speaks the plumbing's.

**Worked example, compiler-checked rather than asserted:** in `gHashTag/trinity`,
`apps/website/public/t27/files/specs/catalog/onboarding.t27` generates
`/llms.txt` and `/agents.t27` byte-identically, gated in CI as
`check:onboarding`. The generator evaluates the spec's own `test` blocks --
`typecheck.ok` stays true for `assert 1 > 2`, so a compiler saying "this parses"
is not a compiler saying "this is true" -- and re-compiles the rendered document
before writing it.

**The full rule lives in exactly one place: the `own-language-first` skill**
(`~/.claude/skills/own-language-first/SKILL.md`). It carries the consent gate for
documents addressed to other people's agents, the six negative controls, and the
`;`-alone-on-a-line trap that silently discards a `module` declaration. This
section is a pointer, not a copy -- the recorded defect in this codebase family
is the hand-copied rule that only two of its three homes knew about.
