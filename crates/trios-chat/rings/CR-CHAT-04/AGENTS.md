# AGENTS — CR-CHAT-04 (padding)

## Identity

You are working on **CR-CHAT-04** — the padding ring of `trios-chat`.

## What this ring does

Implements the four canonical wire-size classes
`{256, 1024, 4096, 16384}` and the `pad_class` / `unpad` primitives.
Implements **R-CHAT-9** (size-class privacy).

## Rules — ABSOLUTE

1. **Silver-tier**. No async, no I/O, no crypto, no randomness.
2. **Only depends on CR-CHAT-00**. No other ring deps allowed.
3. **R-CHAT-9 is law**. Any `pad_class` output length MUST be a member
   of `CLASSES`.
4. **Falsifier-first**. Every code path needs a unit test that proves
   it rejects malformed input.

## You MAY

- Add new size classes only if the `CLASSES` array stays sorted and
  every class is a power-of-two-ish multiple. Document why in `RING.md`.
- Add helper predicates (e.g. `is_padding_class(len)`).

## You MAY NOT

- Pull in `tokio`, `sqlx`, `sea-orm`, `reqwest`, `chacha20poly1305`, or
  any randomness crate.
- Re-export anything from CR-CHAT-01 / 02 / 03 — flow goes the other
  way.

## Build commands

```bash
cargo build -p trios-chat-cr-chat-04
cargo test  -p trios-chat-cr-chat-04
cargo clippy -p trios-chat-cr-chat-04 --all-targets -- -D warnings
```

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
