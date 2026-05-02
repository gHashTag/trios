# TASK.md — SR-00 Scarab Types

**Issue:** Part of #446 (EPIC — E2E TTT Pipeline O(1))
**Soul-name:** TypeArchitect
**Priority:** P1-HIGH
**Kingdom:** Rust
**Lane:** pipeline

---

## Goal

Extract core type definitions from monolith `crates/trios-igla-race/` into a dep-free, serde-serializable ring.

## Acceptance Criteria

- [x] `Cargo.toml` created with serde, chrono, uuid dependencies only
- [x] `RING.md` spec file exists
- [x] `src/lib.rs` implements: `TrialId`, `TrialConfig`, `JobStatus`, `TrialRecord`, `RungResult`, `ExperienceEntry`
- [x] All types are serde-serializable
- [x] Unit tests cover: ID generation, config validation, status checks, record updates
- [x] No business logic in this ring (types only per I3)
- [ ] README.md, TASK.md, AGENTS.md exist (I5 invariant)
- [ ] Cargo workspace includes this ring
- [x] `cargo clippy --all-targets -- -D warnings` passes
- [x] `cargo test --all` passes

## Non-goals

- No database operations (deferred to SR-03 bpb-writer)
- No training logic (deferred to SR-02 trainer-runner)
- No pruning logic (deferred to SR-04 gardener)

## References

- [Issue #446](https://github.com/gHashTag/trios/issues/446)
- [LAWS.md](https://github.com/gHashTag/trios/blob/main/LAWS.md)
- [AGENTS.md](https://github.com/gHashTag/trios/blob/main/AGENTS.md)
- Source: `crates/trios-igla-race/src/neon.rs` (TrialConfig, ExperienceEntry)
