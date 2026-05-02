# RING — SR-MEM-01 (trios-agent-memory)

## Identity

| Field   | Value |
|---------|-------|
| Metal   | 🥈 Silver |
| Package | `trios-agent-memory-sr-mem-01` |
| Sealed  | No |

## Purpose

4-verb KG adapter contract + retry + circuit breaker. Silver-tier: no concrete HTTP / TLS surface — the `trios_kg::KgClient` wrapper lives in a sibling BR-IO ring.

## API Surface (pub)

| Item | Role |
|---|---|
| `KgAdapter<B>` | retry+breaker wrapper over any `KgBackend` |
| `KgBackend` trait | 3 async verbs (`put_triple`, `query_pattern`, `delete_triple`) |
| `AdapterConfig` | tunable retry / breaker policy |
| `AdapterErr` | `BreakerOpen`, `RetryExhausted`, `Client`, `BudgetElapsed` |
| `RecallPattern` | subject/predicate/object optional match |

## Dependencies

- `trios-agent-memory-sr-mem-00` (path) — Triple, TripleId
- `serde`, `serde_json`, `thiserror`, `tracing`
- `tokio` (`macros`, `rt`, `time`, `sync`)

## Laws

- R1 — pure Rust
- L6 — async via trait
- L13 — I-SCOPE: this ring only
- R-RING-DEP-002 — strict dep list above

## Anchor

`φ² + φ⁻² = 3`
