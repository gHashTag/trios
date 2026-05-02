# SR-MEM-00 — Memory Types (anti-amnesia foundation)

**Soul-name:** `Memory Mason` · **Codename:** `LEAD` · **Tier:** 🥉 Bronze · **Kingdom:** Rust

> Closes #449 · Part of #446 · Anchor: `φ² + φ⁻² = 3`

## What this ring does

Dependency-free typed primitives for the agent-memory layer. SR-MEM-00
is the **bottom of the GOLD IV graph** — SR-MEM-01..06 + BR-OUTPUT all
import their wire format from here.

## Public types

| Type | Wire-format role |
|---|---|
| `TripleId([u8; 32])` | content-addressed SHA-256 of `subject \|\| predicate \|\| object` (lowercase-hex JSON, length 64) |
| `AgentRole` | `Scarab`/`Gardener`/`Trainer`/`Doctor`/`Claude`/`Lead` (snake_case) |
| `MemoryKind` | `Working`/`Session`/`LongTerm`/`Episodic`/`Semantic` |
| `ForgetPolicy` | tagged: `GdprByAgent{agent}` / `AgeOlderThan{duration}` / `PredicateMatches{predicate}` |
| `Provenance` | `agent_id`, `task_id` (UUID), `source_sha` (`TripleId`), `ts` |
| `Triple` | `id`, `subject`, `predicate`, `object`, `provenance` |
| `TripleId::from_triple(s,p,o)` | canonical content-address |
| `Triple::new(s,p,o,prov)` | computes `id` from SPO — idempotent insert |

## Content-addressed insert (idempotent)

`TripleId::from_triple(s, p, o)` is pure SHA-256 over the UTF-8
concatenation with no separator. Two calls with identical inputs
always produce the same id — proven by `triple_id_is_content_addressed`
and `triple_idempotent_insert` tests. Any backend (Neon lessons,
`trios-kg`, `zig-knowledge-graph`, HDC) can use the id as a primary
key without coordination.

## Tests (14/14 GREEN)

| Test | Asserts |
|---|---|
| `triple_id_is_content_addressed` | same SPO ⇒ same id |
| `triple_id_distinguishes_inputs` | different SPO ⇒ different id |
| `triple_id_serialises_as_hex64` | 32 bytes → 64 lowercase hex chars |
| `triple_id_rejects_wrong_length` | 31-byte hex fails to deserialise |
| `triple_id_display_lowercase_hex` | stable Display string |
| `agent_role_serialises_snake_case` | wire format |
| `memory_kind_roundtrip` | every variant roundtrips |
| `memory_kind_long_term_is_snake_case` | `LongTerm` → `"long_term"` |
| `forget_policy_gdpr_roundtrip` | tagged variant roundtrips |
| `forget_policy_age_serializes_as_seconds` | `Duration` → integer seconds |
| `forget_policy_predicate_roundtrip` | string variant roundtrips |
| `provenance_roundtrip` | full struct |
| `triple_new_computes_id` | builder derives correct id |
| `triple_idempotent_insert` | byte-equality on re-build |
| `triple_full_roundtrip` | full JSON |

## Dependencies

- `serde` (derive)
- `serde_json`
- `uuid` (`v4`, `serde`)
- `chrono` (`serde`)
- `sha2` — SHA-256 for content addressing

R-RING-DEP-002 — no `tokio`, no `sqlx`, no `reqwest`.

## Build & test

```bash
cargo build  -p trios-agent-memory-sr-mem-00
cargo clippy -p trios-agent-memory-sr-mem-00 --all-targets -- -D warnings
cargo test   -p trios-agent-memory-sr-mem-00
```

## Laws

- L1 ✓ no `.sh`
- L3 ✓ clippy clean
- L6 ✓ no I/O, no async
- L13 ✓ I-SCOPE: only `crates/trios-agent-memory/`
- L14 ✓ `Agent: Memory-Mason` trailer
