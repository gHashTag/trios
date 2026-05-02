# TASK — SR-02

Closes #479 · Part of #446 · Soul: `Constitutional-Cartographer`

## Acceptance

- [x] SR-02 crate `trios-scarab-types-sr-02` created
- [x] Public surface: SoulScarab + SoulScarabType
- [x] Each variant implements `Display` + `as_markdown()`
- [x] README.md with term taxonomy
- [x] AGENTS.md with soul-name + mission
- [x] RING.md with field table + dep budget
- [x] `[dependencies]`: `serde` + SR-00 sibling only (R-RING-DEP-002)
- [x] No `tokio`, no subprocess, no I/O (R-RING-DEP-002 Bronze-tier)
- [x] No `.py` (R-L6-PURE-007)
- [x] No `.sh` (R-L1-ECHO-006)
- [x] `cargo clippy --all-targets -- -D warnings` → 0 warnings
- [x] `cargo test --workspace` → all GREEN
- [x] `Agent: Constitutional-Cartographer` trailer

## R5-honest scope

This ring is part of the parallel-execution foundation enabling 5
agents to start simultaneously via `trinity-bootstrap --codename` (PR
#469). Type-enforced anti-collision: each codename binds to one ring
at compile time.

O-Type + Three-Type binding for Soul-level agents.

## Out of scope (separate follow-ups)

- Wire typed `Term` enum into `trinity-bootstrap` CLI (replaces
  stringly-typed `--codename`). Tracked in
  `.trinity/state/three-roads-479.json` R1.
- Tag Tailscale mesh nodes with one TermScarab so ACL keys derive
  from the typed scarab. Tracked R2.
- SILVER-RING-DR-04 RuleEngine pass over the new rings. Tracked R3.

## Wave-6 debt closure

This file was missing in PR #488 (the original SR-00..04 landing in
Wave 6) — the I5 CI gate flagged it on PR #495 and earlier wave
status comments. This PR closes that debt.
