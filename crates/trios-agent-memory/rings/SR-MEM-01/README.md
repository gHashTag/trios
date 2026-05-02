# SR-MEM-01 — KG Client Adapter (retry + circuit breaker)

**Soul-name:** `Bridge Builder` · **Codename:** `LEAD` · **Tier:** 🥈 Silver

> Closes #453 · Part of #446 · Anchor: `φ² + φ⁻² = 3`

## Honest scope (R5)

This ring ships the adapter **contract + retry/breaker state machine**, not the concrete `trios_kg::KgClient` wrapper. The trade-off is deliberate:

- Issue acceptance asks for `Deps: SR-MEM-00 + trios-kg (path dep) + tokio + tracing`. Pulling `trios-kg` directly into a Silver ring means inheriting `reqwest` + TLS + the full HTTP surface — that violates `R-RING-DEP-002` (no I/O at Silver tier).
- Instead, SR-MEM-01 takes a `KgBackend` **trait object**. The concrete `trios_kg::KgClient` adapter ships in a sibling BR-IO ring (`crates/trios-agent-memory/adapters/trios_kg/`) and is the only place that links `reqwest`. SR-04 (gardener) and SR-03 (bpb-writer) follow the same pattern.
- Net effect: same 4 verbs (`remember_triple`, `recall_by_pattern`, `supersede`, `tombstone`) reach the live KG, but Silver-tier rings can mock the backend in unit tests without any HTTP server.

The 4-verb contract, retry config (max 3 attempts, 30 s budget, exp backoff), and breaker policy (5-of-60 s ⇒ open, 30 s half-open) match the issue 1:1.

## API

```rust
pub struct KgAdapter<B: KgBackend>;

impl<B: KgBackend> KgAdapter<B> {
    pub fn new(backend: B) -> Self;                        // default policy
    pub fn with_config(backend: B, config: AdapterConfig) -> Self;

    pub async fn remember_triple(&self, t: &Triple) -> Result<TripleId, AdapterErr>;  // idempotent on SHA-256
    pub async fn recall_by_pattern(&self, p: &RecallPattern, budget: usize) -> Result<Vec<Triple>, AdapterErr>;
    pub async fn supersede(&self, old: TripleId, new: &Triple) -> Result<TripleId, AdapterErr>;
    pub async fn tombstone(&self, id: TripleId) -> Result<(), AdapterErr>;
}

pub trait KgBackend: Send + Sync { /* 3 async fns: put_triple, query_pattern, delete_triple */ }

pub struct RecallPattern { subject, predicate, object: Option<String> }
pub struct AdapterConfig { max_attempts, call_budget, backoff_*, breaker_* }
pub enum  AdapterErr { BreakerOpen, RetryExhausted, Client, BudgetElapsed }
```

## Retry & breaker semantics

- **Retry**: every call gets up to `max_attempts=3` tries with exponential backoff (`backoff_initial=200 ms`, multiplier 2.0). Total per-call deadline is `call_budget=30 s`.
- **Circuit breaker**: 5 consecutive failed *calls* within `breaker_window=60 s` trips the breaker. Open state refuses new calls with `AdapterErr::BreakerOpen` for `breaker_open_duration=30 s`. After that, the breaker enters half-open (one probe). Success on the probe closes it; failure re-opens.
- Every async fn uses `tracing::instrument` (matches `trios-kg/src/client.rs` style).

## Tests (15/15 GREEN)

| Group | Tests |
|---|---|
| Idempotency | `remember_triple_idempotent_on_id` |
| Retry | `retry_succeeds_within_budget`, `retry_exhaustion_returns_error` |
| Circuit breaker | `breaker_opens_after_threshold_failures`, `breaker_half_opens_after_window` |
| Recall | `recall_by_pattern_matches_predicate`, `recall_respects_budget` |
| Supersede / Tombstone | `supersede_inserts_then_deletes`, `tombstone_removes_triple` |
| Pattern semantics | `pattern_matches_all_when_empty`, `pattern_rejects_mismatch` |
| Config defaults | `config_defaults_match_acceptance_criteria` |

The mock backend is in-memory + injectable failures (`fail_for(n)`).

## Dependencies

- `trios-agent-memory-sr-mem-00` (path) — `Triple`, `TripleId`
- `serde`, `serde_json`
- `thiserror` — `AdapterErr`
- `tracing` — `instrument` macros
- `tokio` (`macros`, `rt`, `time`, `sync`) — async + breaker timer

No `reqwest`, no `trios-kg` linked at this level. The concrete adapter sits in BR-IO and brings those in.

## Build & test

```bash
cargo build  -p trios-agent-memory-sr-mem-01
cargo clippy -p trios-agent-memory-sr-mem-01 --all-targets -- -D warnings
cargo test   -p trios-agent-memory-sr-mem-01
```

## Laws

L1 ✓ · L3 ✓ · L4 ✓ · L13 ✓ · L14 ✓ · R-RING-DEP-002 ✓ · R-RING-FACADE-001 ✓ · I5 ✓
