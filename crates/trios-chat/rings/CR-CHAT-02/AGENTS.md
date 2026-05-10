# AGENTS — CR-CHAT-02 (ratchet)

## Identity

You are working on **CR-CHAT-02** — the ratchet ring of `trios-chat`.

## What this ring does

Forward-secret chain key + replay-resistant counter window + skipped
keys + DH root rotation. Implements **R-CHAT-2**.

## Rules — ABSOLUTE

1. **Silver-tier**. No async, no I/O, no DB.
2. **R-CHAT-2 forward secrecy**. `next_message_key` MUST overwrite
   `chain_key` in place. Never keep the old chain key.
3. **R-CHAT-4 / no per-message Ed25519**. The chain provides the AEAD
   key only. Authentication is via the AEAD tag, not via signatures.
4. **Bounded memory**. `skipped` MUST never grow beyond
   `SKIPPED_KEYS_CAP`. Add tests when adjusting this.
5. **Replay window = 64**. Don't widen it without updating the law
   table in CR-CHAT-LAWS.

## You MAY

- Mix ML-KEM-768 shared secret into `dh_step` once CR-CHAT-01 wires
  the concrete `ml-kem` crate. Update the salt label and bump
  PROTOCOL_VERSION.
- Add diagnostic helpers (e.g. `Chain::is_fresh()`).

## You MAY NOT

- Add `tokio`, `sqlx`, or any storage.
- Allow `RootKey` / `ChainKey` to derive `Serialize` — the ratchet
  state is intentionally non-persistable.

## Build commands

```bash
cargo build -p trios-chat-cr-chat-02
cargo test  -p trios-chat-cr-chat-02
cargo clippy -p trios-chat-cr-chat-02 --all-targets -- -D warnings
```

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
