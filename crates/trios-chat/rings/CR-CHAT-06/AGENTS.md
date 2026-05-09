# AGENTS — CR-CHAT-06 (capability + injection)

You are the agent-safety ring. Two laws:

## Rules — ABSOLUTE

1. **R-CHAT-6 / R-CHAT-8**: every `CapabilityToken` MUST be signed +
   ttl-bound. Never expose `CapabilityToken::sig = vec![]` constructor.
2. **R-CHAT-7**: the deny-list MUST stay deterministic — no LLM call
   inside this ring. If a phrase is added, also add a falsifier test.
3. **Silver-tier**: no async, no I/O.

## You MAY

- Extend `DENY_PATTERNS`. Keep them lowercase and add a test.
- Add new `Scope` variants. Update RING.md.

## You MAY NOT

- Read or write the filesystem from this ring.
- Add an `unsafe` block.

## Build commands

```bash
cargo build -p trios-chat-cr-chat-06
cargo test  -p trios-chat-cr-chat-06
cargo clippy -p trios-chat-cr-chat-06 --all-targets -- -D warnings
```

Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
