# trios 🔱

> **Trinity Git Orchestrator** — MCP server for AI agents to control Git & GitButler

[![CI](https://github.com/gHashTag/trios/actions/workflows/ci.yml/badge.svg)](https://github.com/gHashTag/trios/actions)

## What is trios?

`trios` is a minimal MCP (Model Context Protocol) server that allows AI agents (like BrowserOS Assistant) to control Git repositories and GitButler virtual branches **without touching any UI**.

The agent sends MCP tool calls → trios executes via `git2-rs` (stable) + `gitbutler-cli` (GB features) → GitButler UI updates automatically via FSNotify.

## Architecture

```
BrowserOS Agent → trios-server (9005) → trios-git (git2-rs)
                                      → trios-gb  (gitbutler-cli)
                                                  → .git/ → GitButler UI
```

## Quick Start

```bash
# Clone
git clone https://github.com/gHashTag/trios
cd trios

# Build
cargo build

# Run MCP server
cargo run -p trios-server
# Server starts at http://localhost:9005

# Test
cargo test
```

## MCP API

```bash
# Stage files
curl -X POST http://localhost:9005/mcp/tools/call \
  -H 'Content-Type: application/json' \
  -d '{"name": "git_stage_files", "input": {"repo_path": "/path/to/repo", "paths": ["src/main.rs"]}}'

# Commit
curl -X POST http://localhost:9005/mcp/tools/call \
  -H 'Content-Type: application/json' \
  -d '{"name": "git_commit", "input": {"repo_path": "/path/to/repo", "message": "feat: add feature"}}'
```

## Crates

| Crate | Purpose |
|-------|--------|
| `trios-core` | Traits, types, shared abstractions |
| `trios-git` | Git operations via libgit2 |
| `trios-gb` | GitButler CLI wrapper with fallback |
| `trios-server` | Axum MCP HTTP server on port 9005 |

## Laws

See [CLAUDE.md](./CLAUDE.md) for full rules. Summary:
- NO `.sh` files
- `cargo clippy -- -D warnings` = 0
- `cargo test` before merge
- Every PR closes an issue

## Related

- [gHashTag/t27](https://github.com/gHashTag/t27) — Trinity math research
- [gHashTag/BrowserOS](https://github.com/gHashTag/BrowserOS) — Agent that uses trios
- [gHashTag/gitbutler](https://github.com/gHashTag/gitbutler) — GitButler fork
