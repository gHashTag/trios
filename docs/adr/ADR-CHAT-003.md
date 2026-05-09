# ADR-CHAT-003: No per-message Ed25519 signatures (deniability)

- **Status**: Accepted (Trinity Secure Chat EPIC trinity-fpga#28, scaffold)
- **Date**: 2026-05-09
- **Anchor**: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

## Context

Per-message signatures enable forwarded transcripts to convict a sender — breaks deniable authentication.

## Decision

Authenticate messages with HMAC derived from chain-key; sign only prekey bundles.

## Consequences

Pros: deniability preserved. Cons: receivers cannot prove a forwarded message to a 3rd party (intended).

## References

- [https://petsymposium.org/popets/2025/popets-2025-0018.pdf](https://petsymposium.org/popets/2025/popets-2025-0018.pdf)
- Design doc: [`/docs/chat/trinity-chat-design.md`](../chat/trinity-chat-design.md)
- Constitutional law: see [`crates/trios-chat/src/r_chat.rs`](../../crates/trios-chat/src/r_chat.rs)
