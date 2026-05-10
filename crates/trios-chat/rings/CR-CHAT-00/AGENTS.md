# AGENTS.md — CR-CHAT-00 (trios-chat)

## Identity

- Ring: CR-CHAT-00
- Package: `trios-chat-cr-chat-00`
- Role: chat wire-format primitives + R-CHAT law table
- Codename: `LEAD`

## What this ring does

`SessionId`, `Counter`, `DestHash`, `EnvelopeMeta`, `Error`, `Result`,
`chat_laws()`. Pure data + serde. Imported by every other CR-CHAT-* and
BR-IO-CHAT-* ring.

## Rules (ABSOLUTE)

- R1   — pure Rust
- L6   — no I/O, no async
- L13  — I-SCOPE: only this ring
- R-RING-DEP-002 — deps = `serde + serde_json + thiserror` (nothing else)
- **R-CHAT law table is canonical** — adding/removing a law here is a
  cross-repo wire-format break and must be paired with EPIC update on
  trinity-fpga#28.

## You MAY

- ✅ Add new `Error` variants (non-breaking)
- ✅ Add new `EnvelopeMeta` field with `#[serde(default)]`
- ✅ Add tests, especially serde roundtrip property tests

## You MAY NOT

- ❌ Change wire format of `SessionId` / `Counter` / `DestHash` once shipped
- ❌ Add tokio / sqlx / sea-orm / reqwest
- ❌ Drop a law from `chat_laws()` once shipped
- ❌ Rename a public type (downstream rings break silently)

## Build

```bash
cargo build  -p trios-chat-cr-chat-00
cargo clippy -p trios-chat-cr-chat-00 --all-targets -- -D warnings
cargo test   -p trios-chat-cr-chat-00
```

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
