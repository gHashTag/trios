# LAWS.md — trios Constitutional Document

## SUPREMACY

This document is the supreme law of the trios repository.
All code, commits, PRs, and CI workflows must comply with these laws.

---

## L1: NO .sh files
All automation must be Rust binaries or TypeScript. Shell scripts (.sh) are banned.

## L2: Every PR closes an issue
Every PR description MUST contain `Closes #N`. No orphan PRs.

## L3: clippy zero warnings
```bash
cargo clippy -- -D warnings
```
Must pass before any merge.

## L4: Tests before merge
```bash
cargo test
```
All tests must pass. New code requires new tests.

## L5: Port 9005 is trios-server
The MCP server always runs on `0.0.0.0:9005`. Never change this without a migration plan.

## L6: Fallback required for GB tools
`trios-gb` tools must gracefully return `Err` (not panic) if `gitbutler-cli` is not found.

## L7: Experience log
Every significant task writes a line to `.trinity/experience/`.

## L8: PUSH FIRST LAW
Every file change = immediate commit + push.

---

*LAWS.md v1.0 — Constitutional baseline*
