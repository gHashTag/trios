# RING — CR-CHAT-01 (trios-chat)

## Identity

| Field   | Value |
|---------|-------|
| Tier    | 🥈 Silver (Core ring) |
| Package | `trios-chat-cr-chat-01` |
| Path    | `crates/trios-chat/rings/CR-CHAT-01/` |
| Sealed  | No |

## Purpose

The first crypto ring. Two tightly-coupled chat primitives live here:

1. **`identity`** — `Identity::generate()`, `PrekeyBundle` build/verify,
   `safety_number()`. Implements **R-CHAT-2** (hybrid X25519 ⊕ ML-KEM-768
   from day 1) and **R-CHAT-4** (sign only the bundle, never per
   message).
2. **`sealed`** — `SealedEnvelope::seal/unseal`, `dest_hash`. Implements
   **R-CHAT-3** (the mesh sees only `dest_hash[16]` + padded envelope).

They share enough types (`x25519-dalek::PublicKey`, the
`Identity` struct, the canonical KDF rule) that splitting them into
two rings would force CR-CHAT-01a to re-export half of CR-CHAT-01b and
back. Keeping them together preserves the **single-canonical-KDF** rule.

## Public API

| Item | Role |
|---|---|
| `Identity` | Long-term Ed25519 + X25519 prekey + ML-KEM-768 seed |
| `PrekeyBundle` / `PrekeyBundleBody` | Signed prekey for publication |
| `MLKEM_PUB_LEN` / `MLKEM_SEC_LEN` | FIPS 203 sizes |
| `SealedEnvelope` | `{ dest_hash, src_x25519_pub, nonce, ciphertext }` |
| `dest_hash(&PublicKey)` | 16-byte routing hint |

Re-exported flat at the crate root for ergonomic consumer use.

## Dependencies

| Dep | Why |
|---|---|
| `trios-chat-cr-chat-00` | `Error`, `Result` |
| `trios-chat-cr-chat-04` | `pad_class` / `unpad` (R-CHAT-9) |
| `ed25519-dalek` | Long-term signing key |
| `x25519-dalek`  | Prekey + sealed-sender ECDH |
| `chacha20poly1305` | AEAD |
| `sha2`           | KDF + dest-hash |
| `rand_core`      | OsRng |
| `zeroize`        | Drop-zeroes for secret keys |
| `serde`          | Bundle wire format |

No `tokio`, no `sqlx`, no `reqwest`. Silver-tier purity preserved.

## Invariants

- **R-CHAT-2** — every prekey bundle ships an ML-KEM-768 placeholder
  alongside X25519, so PQ migration is wire-compatible.
- **R-CHAT-3** — `dest_hash(recipient_pub)` is the **only** routing
  field a mesh observer sees beyond ciphertext.
- **R-CHAT-4** — `Identity::sign` is exposed but messages MUST not be
  signed per-message; the prekey bundle is the only signed artefact.
- **R-CHAT-9** — sealed ciphertext length is always `class + 16` for
  one of the 4 canonical padding classes.
- **dest-hash unlinkability** — `dest_hash` is deterministic on
  recipient pub but pseudo-random across different keys (covered by
  test `dest_hash_differs_for_different_keys`).

## Tests

13 unit tests (6 identity + 6 sealed + 1 padding-class assertion).

## Sibling Bronze

None — the only I/O sealed envelopes need is "write bytes to mesh",
which lives in `trios-mesh-node` already.

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
