# AGENTS — CR-CHAT-01 (identity + sealed)

## Identity

You are working on **CR-CHAT-01** — the first crypto ring of `trios-chat`.

## What this ring does

- Long-term identity (Ed25519) + prekey bundle (X25519 + ML-KEM-768
  placeholder), implementing **R-CHAT-2 / R-CHAT-4**.
- Sealed-sender envelope, implementing **R-CHAT-3**.

## Rules — ABSOLUTE

1. **Silver-tier**. No `tokio`, no `sqlx`/`sea-orm`, no `reqwest`, no
   filesystem.
2. **R-CHAT-4**. Do **NOT** add any `Identity::sign_message(payload)`
   helper that is called per-message. Per-message authentication is
   the ratchet's MAC key, not Ed25519. `Identity::sign` exists only
   for bundle-time / capability-time signing.
3. **R-CHAT-3 / dest-hash purity**. The 16-byte `dest_hash` MUST be
   the only field a mesh observer sees beyond AEAD bytes. Do NOT add
   any field to `SealedEnvelope` outside the canonical four
   (`dest_hash`, `src_x25519_pub`, `nonce`, `ciphertext`).
4. **Single canonical KDF**. Both directions of a sealed channel
   compute the same key by sorting public keys lexicographically. Do
   NOT add a per-direction KDF — that breaks A↔B symmetry.
5. **ML-KEM stays opaque**. Until CR-CHAT-02 wires the concrete
   `ml-kem` crate, public bytes are SHA-256(seed) repeated — never
   reveal that this is a placeholder via the public API.

## You MAY

- Add additional helper functions on `Identity` (e.g.
  `from_bytes` / `to_bytes`) — they MUST zeroize on drop.
- Add new falsifier tests.
- Re-export `x25519_dalek::{PublicKey, StaticSecret}` if a downstream
  ring asks; today they aren't re-exported, callers import them
  directly.

## You MAY NOT

- Add a `tokio::main` test.
- Add a feature flag `sqlx`.
- Pull in `serde_json` here — the bundle is bincode/serde-cbor terrain
  for now (handled by `serde` only).

## Build commands

```bash
cargo build -p trios-chat-cr-chat-01
cargo test  -p trios-chat-cr-chat-01
cargo clippy -p trios-chat-cr-chat-01 --all-targets -- -D warnings
```

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
