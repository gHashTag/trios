# TASK — SR-02 (trios-igla-race-pipeline)

## Status: SCAFFOLDED

Part of #446

## Completed

- [x] Ring directory created
- [x] Cargo.toml — deps: SR-00, SR-01, serde, serde_json, thiserror, chrono; `tch` feature
- [x] README.md, TASK.md, AGENTS.md present (Invariant I5)

## Open

- [ ] `TrainerRunner` struct — wraps a `Scarab`, drives inner TTT loop
- [ ] `RunResult` — final BPB + best_step + elapsed
- [ ] Emit `Heartbeat` every N steps via callback / channel
- [ ] CPU-only stub trainer (no libtorch) for unit tests
- [ ] `tch` feature gate — real libtorch backend behind feature flag
- [ ] Unit tests: stub runner completes 3 steps, emits 3 heartbeats
- [ ] clippy clean (`--all-targets -- -D warnings`)
- [ ] Seal ring (Bronze → Silver promotion)

## Next ring

SR-03 bpb-writer — consumes `BpbSampleRow` output from SR-02.
