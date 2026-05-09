# RING — CR-CHAT-02 (trios-chat)

## Identity

| Field   | Value |
|---------|-------|
| Tier    | 🥈 Silver (Core ring) |
| Package | `trios-chat-cr-chat-02` |
| Path    | `crates/trios-chat/rings/CR-CHAT-02/` |
| Sealed  | No |

## Purpose

The ratchet ring. Implements **R-CHAT-2** (forward-secrecy + future
post-compromise security) via a Signal-style Double Ratchet skeleton:

- `RootKey` / `ChainKey` — KDF-chained 32-byte secrets.
- `MessageKey` — derived from chain-key step.
- `Chain::send_next` / `recv_accept` — sender / receiver halves.
- `Chain::dh_step` — root-key rotation on a fresh X25519 DH (PQ-KEM
  layer to be mixed in by the L-CHAT-2 follow-up).

Replay window of 64 counters; skipped-keys cache capped at
`SKIPPED_KEYS_CAP = 1024` so an attacker who sprays a
counter-jump cannot exhaust memory.

## Public API

| Item | Role |
|---|---|
| `RootKey([u8; 32])` | rotates only on `dh_step` |
| `ChainKey([u8; 32])` | rotates on every message |
| `MessageKey { key, nonce, counter }` | feeds AEAD in CR-CHAT-01 |
| `Chain::from_root` | start a fresh direction |
| `Chain::send_next` | sender half |
| `Chain::recv_accept` | receiver half (replay-checked) |
| `Chain::dh_step` | root-key rotation |
| `Chain::take_skipped` / `skipped_len` | out-of-order cache helpers |
| `SKIPPED_KEYS_CAP` | memory ceiling |

## Dependencies

| Dep | Why |
|---|---|
| `trios-chat-cr-chat-00` | `Error`, `Result` |
| `hkdf`           | KDF chain |
| `sha2`           | HKDF underlying hash |
| `x25519-dalek`   | DH-step |
| `zeroize`        | Secret wipe on drop |

No async, no I/O, no DB.

## Invariants

- **Strict monotonicity**: `counter` strictly increases per direction,
  never repeats.
- **Replay window**: any counter ≥ 64 behind `counter` is rejected.
- **Skipped-keys cap**: cache size ≤ `SKIPPED_KEYS_CAP`, even under an
  adversarial future-counter spray (verified by
  `skipped_keys_capped_under_adversarial_jump`).
- **DH-step symmetry**: Alice and Bob, given the same prior root + each
  other's prekey, derive identical post-step roots and chains.

## Tests

10 unit tests including 1 explicit memory-cap falsifier.

## Sibling Bronze

None — chain state is in-memory by design (per Signal threat model;
persistence of chain state would expand the compromise surface).

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
