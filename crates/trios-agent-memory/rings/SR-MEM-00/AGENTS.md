# AGENTS.md — SR-MEM-00 (trios-agent-memory)

## Identity

- Ring: SR-MEM-00
- Package: `trios-agent-memory-sr-mem-00`
- Role: anti-amnesia wire-format primitives
- Soul-name: `Memory Mason`
- Codename: `LEAD`

## What this ring does

`TripleId`, `AgentRole`, `MemoryKind`, `ForgetPolicy`, `Provenance`, `Triple`. Pure data + content-address + serde.

## Rules (ABSOLUTE)

- R1   — pure Rust
- L6   — no I/O, no async
- L13  — I-SCOPE: only this ring
- R-RING-DEP-002 — deps = `serde + serde_json + uuid + chrono + sha2`
- **Content-addressed identity** — `TripleId::from_triple` MUST stay deterministic. Any change to the hash input (adding separators, normalisation, etc.) is a wire-format break and needs a paired migration.

## You MAY

- ✅ Add new `AgentRole` / `MemoryKind` variants (non-breaking)
- ✅ Add new `ForgetPolicy` tagged variants
- ✅ Add `Triple` field with `#[serde(default)]` (non-breaking)
- ✅ Add tests, especially property tests for idempotency

## You MAY NOT

- ❌ Change `TripleId::from_triple` hash recipe
- ❌ Add tokio / sqlx / reqwest
- ❌ Drop an `AgentRole` variant once shipped
- ❌ Break the `duration_secs` integer-seconds wire format

## Build

```bash
cargo build  -p trios-agent-memory-sr-mem-00
cargo clippy -p trios-agent-memory-sr-mem-00 --all-targets -- -D warnings
cargo test   -p trios-agent-memory-sr-mem-00
```

## Anchor

`φ² + φ⁻² = 3`
