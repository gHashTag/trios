# CLAUDE.md — Instructions for Claude Code and Autonomous Agents

## Overview

This repository is the **Trinity Git Orchestrator** — an MCP server for AI agents to control Git and GitButler via libgit2 + CLI.

**Repo:** https://github.com/gHashTag/trios

---

## Project Structure

```
trios/
├── crates/                 # Rust crates
│   ├── trios-server/      # Axum server (port 9005)
│   ├── trios-git/         # git2-rs bindings
│   ├── trios-gb/          # GitButler CLI integration
│   └── trios-ext/         # WASM browser extension
├── .trinity/
│   ├── prompts/           # Agent dispatch prompts
│   │   └── agent-dispatch.md
│   └── experience/        # Agent institutional memory
├── LAWS.md                # Constitutional document (v2.0)
└── CLAUDE.md              # This file
```

---

## Mandatory Read Order

1. **[LAWS.md](LAWS.md)** — Constitutional stack, immutable laws L1-L9
2. **[.trinity/prompts/agent-dispatch.md](.trinity/prompts/agent-dispatch.md)** — PHI LOOP workflow
3. **This file** — Project conventions

---

## Agent Dispatch

For autonomous agents, use the **one-shot dispatch prompt** located at:

**[`.trinity/prompts/agent-dispatch.md`](.trinity/prompts/agent-dispatch.md)**

This prompt contains:
- Complete PHI LOOP workflow (11 steps)
- All LAWS embedded (L1-L9)
- DONE checklist that blocks premature victory
- HEARTBEAT format for status reporting

**Usage:** Replace `{{ISSUE_NUMBER}}`, `{{ISSUE_TITLE}}`, and let the agent choose `{{YOUR_SOUL_NAME}}` in step NAME.

---

## Engineering Workflow

### Development

```bash
# Build all crates
cargo build --all

# Run tests
cargo test --all

# Check for warnings (MUST be zero before commit)
cargo clippy --all-targets
```

### Commit Protocol

**L6 (PUSH_FIRST) is absolute:** No commit is "done" until:
1. `cargo clippy --all-targets` = 0 warnings
2. `cargo test --all` = all pass
3. `git status` = 0 modified files
4. Commit visible on `github.com/gHashTag/trios`

### PR Requirements

Every PR must include:
- `Closes #{{ISSUE_NUMBER}}` in body
- Zero clippy warnings
- All tests passing
- Link to issue

---

## Architecture

### MCP Server

```
BrowserOS Agent
    │ MCP tool call (A2A)
    ▼
trios-server (port 9005, Axum)
    ├── trios-git   (git2-rs)  ← stable git ops
    └── trios-gb    (CLI)      ← GitButler virtual branches
```

### Browser Extension

- Location: `crates/trios-ext/extension/` (single extension tree)
- No `/extension` at root
- WASM compilation target
- Rust only (no handwritten JS per L8)

---

## Quick Reference

### Common Commands

```bash
# Start MCP server
cargo run --bin trios-server

# Run GitButler integration tests
cargo test --package trios-gb

# Build WASM extension
wasm-pack build --target web crates/trios-ext
```

### Key Concepts

- **PHI LOOP:** 11-step autonomous workflow
- **Soul Name:** Agent identity (e.g., "RustWeaver", "LawGuardian")
- **HEARTBEAT:** Status report format for issue comments
- **DONE Checklist:** All 8 checkboxes must be true before claiming completion

---

## Getting Help

- Issues: https://github.com/gHashTag/trios/issues
- Discord: (add when available)
- Documentation: See `docs/` directory (when created)

---

## References

- **[LAWS.md](LAWS.md)** — Constitutional document v2.0
- **[Agent Dispatch Prompt](.trinity/prompts/agent-dispatch.md)** — One-shot briefing
- **[.trinity/experience/](.trinity/experience/)** — Agent learnings

---

**Last Updated:** 2026-05-02
