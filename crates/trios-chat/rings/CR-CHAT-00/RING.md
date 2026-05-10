# RING — CR-CHAT-00 (trios-chat)

## Identity

| Field   | Value |
|---------|-------|
| Tier    | 🥉 Silver (Core ring) |
| Package | `trios-chat-cr-chat-00` |
| Path    | `crates/trios-chat/rings/CR-CHAT-00/` |
| Sealed  | No |

## Purpose

Bottom of the trios-chat dependency graph. Defines the wire-format
primitives every other chat ring imports — `SessionId`, `Counter`,
`DestHash`, `EnvelopeMeta`, the `Error` / `Result` pair, and the
canonical `R-CHAT-1..12` law table.

No I/O. No async. No crypto. Pure data + serde.

## Why CR-CHAT-00 is the bottom

Every backend (in-memory / SeaORM-Postgres / Neon / future Tailscale
mesh) and every protocol layer (sealed / ratchet / group / injection)
must speak the same envelope shape. Keeping CR-CHAT-00 dep-free
guarantees the whole `trios-chat` ring graph compiles in one pass.

## Public API

| Item | Role |
|---|---|
| `SessionId([u8; 32])` | opaque session identity |
| `Counter(u64)`        | strictly-monotone ratchet counter |
| `DestHash([u8; 16])`  | routing hint per **R-CHAT-3** |
| `EnvelopeMeta`        | non-secret metadata travelling alongside ciphertext |
| `Error`               | crate-wide error enum (thiserror) |
| `Result<T>`           | shorthand `Result<T, Error>` |
| `chat_laws()`         | static `R-CHAT-1..12` law table |

## Dependencies

- `serde`, `serde_json` — wire format
- `thiserror` — error derive

## Invariants (R-RING-DEP-002)

- ❌ no tokio
- ❌ no sqlx / sea-orm / reqwest / hyper
- ❌ no x25519 / ed25519 / chacha20poly1305 / sha2 / hkdf
- ✅ `cargo check --target wasm32-unknown-unknown` passes

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
