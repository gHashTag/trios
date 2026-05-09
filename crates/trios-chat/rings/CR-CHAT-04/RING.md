# RING — CR-CHAT-04 (trios-chat)

## Identity

| Field   | Value |
|---------|-------|
| Tier    | 🥈 Silver (Core ring) |
| Package | `trios-chat-cr-chat-04` |
| Path    | `crates/trios-chat/rings/CR-CHAT-04/` |
| Sealed  | No |

## Purpose

Wire-format **padding** layer for Trinity Secure Chat. Implements
**R-CHAT-9** (size-class privacy) — every ciphertext on the wire is one
of four fixed sizes `{256, 1024, 4096, 16384}` bytes so an observer
cannot discriminate users by message-length distribution.

Pure layout: no crypto, no I/O, no async, no randomness.

## Public API

| Item | Role |
|---|---|
| `CLASSES: [usize; 4]` | the four canonical padding sizes |
| `MAX_PAYLOAD: usize`  | largest payload accepted = 16380 |
| `pad_class(&[u8]) -> Vec<u8>` | pad into smallest containing class |
| `unpad(&[u8]) -> Result<&[u8]>` | parse a padded buffer back to its payload |

## Dependencies

| Dep | Why |
|---|---|
| `trios-chat-cr-chat-00` | `Error`, `Result` |

No serde, no async, no I/O — keeps this ring re-usable in WASM and `no_std`-able later if needed.

## Invariants

- `R-CHAT-9` — `pad_class(p).len() ∈ CLASSES` for every input.
- `pad_class` and `unpad` are total inverses for `payload.len() ≤ MAX_PAYLOAD`.
- `unpad` rejects any buffer whose length is not in `CLASSES`.
- `unpad` rejects any declared length that exceeds the buffer.

## Tests

7 unit tests — class boundaries, round-trip, two falsifier rejection
cases, size-leak-resistance, short-buffer rejection,
max-payload bound.

## Sibling Bronze

None. This ring is pure layout — no I/O variant exists.

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
