# RING — SR-02 (trios-igla-race-pipeline)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Package | trios-igla-race-pipeline-sr-02 |
| Sealed | No |

## Purpose

Trainer-runner scaffold: E2E TTT O(1) per-chunk core. **Scaffold only** — the ring currently contains a `Cargo.toml` (deps on SR-00 types and SR-01 strategy-queue) and is NOT yet a workspace member; no `src/` exists. Logic lands here when the trainer-runner is migrated from the pipeline scripts.

## API Surface (pub)

| Item | Role |
|------|------|
| — | no public API yet (scaffold, no `src/`) |

## Laws

- R1 / R5 / R9: Ring isolation, no sibling imports, parent re-exports only
- I5: README + TASK + AGENTS present in every ring
