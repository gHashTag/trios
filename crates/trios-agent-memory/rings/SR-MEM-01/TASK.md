# TASK — SR-MEM-01 (trios-agent-memory)

## Status: DONE ✅ (Silver contract); concrete `trios_kg::KgClient` adapter is BR-IO follow-up

Closes #453 · Part of #446

## Completed

- [x] Ring at `rings/SR-MEM-01/` with I5 trinity (`README.md`, `TASK.md`, `AGENTS.md`, `RING.md`, `Cargo.toml`, `src/lib.rs`)
- [x] `KgAdapter<B: KgBackend>` with all 4 verbs (`remember_triple`, `recall_by_pattern`, `supersede`, `tombstone`)
- [x] `KgBackend` trait — 3 async fns (`put_triple`, `query_pattern`, `delete_triple`) — object-safe + `Send + Sync`
- [x] `RecallPattern { subject, predicate, object: Option<String> }` with `matches()`
- [x] Retry: exponential backoff, `max_attempts=3`, `call_budget=30 s`
- [x] Circuit breaker: `breaker_threshold=5`, `breaker_window=60 s`, `breaker_open_duration=30 s`, half-open probe after open window elapses
- [x] Every async fn uses `tracing::instrument`
- [x] 15 unit tests — idempotency, retry success, retry exhaustion, breaker open, half-open probe, recall match, recall budget, supersede, tombstone, pattern matching, config defaults
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `Agent: Bridge-Builder` trailer (L14)

## Honest scope decision (R5)

Issue acceptance lists `trios-kg (path dep)` among the required deps. Pulling `trios-kg` (+ its `reqwest` / TLS transitive surface) into a Silver-tier ring violates `R-RING-DEP-002` — this is the same principle why SR-03 (`BpbSink`) and SR-04 (`GardenerSink`) ship as trait contracts, with the concrete HTTP/SQL adapter in a BR-IO ring. This PR follows that established pattern:

- **SR-MEM-01 (this PR):** 4-verb contract + retry/breaker + in-memory mock tests (Silver-tier, no I/O).
- **BR-IO `trios_kg_adapter` (follow-up PR):** `impl KgBackend for TriosKgBackend` that wraps `trios_kg::KgClient`. One-file ring, straightforward once SR-MEM-01 is in main.

The integration test against a stub server is deferred to the BR-IO ring (where `trios-kg` is actually linked); SR-MEM-01's in-memory `MockBackend` covers every retry / breaker state transition.

## Open (handed to next rings)

- [ ] BR-IO `trios_kg_adapter` — concrete `KgBackend` impl over `trios_kg::KgClient`
- [ ] SR-MEM-02..05 — higher-level recall strategies (HDC cosine, semantic search)
- [ ] BR-OUTPUT `AgentMemory` trait assembler (#461) — depends on SR-MEM-01 + SR-MEM-05

## Next ring

BR-IO `trios_kg_adapter` (new issue).
