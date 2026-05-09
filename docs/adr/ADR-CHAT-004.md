# ADR-CHAT-004: Fixed padding classes {256, 1024, 4096, 16384}

- **Status**: Accepted (Trinity Secure Chat EPIC trinity-fpga#28, scaffold)
- **Date**: 2026-05-09
- **Anchor**: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

## Context

Variable ciphertext lengths leak typing patterns and message types over a public mesh.

## Decision

All sealed envelopes are padded to the smallest of four fixed classes; >16380 B splits into multiple ratchet messages.

## Consequences

Pros: 4-class size leak only. Cons: small messages cost ≥256 B; large media bypassed via separate file channel.

## References

- [https://signal.org/docs/](https://signal.org/docs/)
- Design doc: [`/docs/chat/trinity-chat-design.md`](../chat/trinity-chat-design.md)
- Constitutional law: see [`crates/trios-chat/src/r_chat.rs`](../../crates/trios-chat/src/r_chat.rs)
