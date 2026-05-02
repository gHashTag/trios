# TASK — SR-MEM-00 (trios-agent-memory)

## Status: DONE ✅

Closes #449 · Part of #446

## Completed

- [x] Outer GOLD IV crate `trios-agent-memory` scaffolded
- [x] SR-MEM-00 ring with I5 (`README.md`, `TASK.md`, `AGENTS.md`, `RING.md`, `Cargo.toml`, `src/lib.rs`)
- [x] `TripleId([u8; 32])` content-addressed via SHA-256
- [x] `TripleId::from_triple(s, p, o)` idempotent
- [x] Custom hex-64 serde for `TripleId` (length-validated on deserialise)
- [x] `AgentRole` enum (Scarab / Gardener / Trainer / Doctor / Claude / Lead)
- [x] `MemoryKind` enum (Working / Session / LongTerm / Episodic / Semantic)
- [x] `ForgetPolicy` tagged enum (GdprByAgent / AgeOlderThan / PredicateMatches)
- [x] `Duration` serialises as integer seconds (custom `duration_secs` module)
- [x] `Provenance { agent_id, task_id, source_sha, ts }`
- [x] `Triple { id, subject, predicate, object, provenance }` + `Triple::new()` builder
- [x] 14 unit tests — content-address, hex roundtrip, every enum variant, idempotent insert
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `Agent: Memory-Mason` trailer (L14)

## Open (handed to next rings)

- [ ] SR-MEM-01 kg-client-adapter (#453) — retry / circuit-breaker over `trios-kg`
- [ ] SR-MEM-05 episodic-bridge (#455) — `lessons.rs` + HDC ↔ KG
- [ ] BR-OUTPUT — `AgentMemory` trait assembler (#461)

## Next ring

SR-MEM-01 kg-client-adapter (#453).
