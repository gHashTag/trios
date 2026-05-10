# ADR-CHAT-008: No SGX/SEV trusted execution dependency

- **Status**: Accepted (Trinity Secure Chat EPIC trinity-fpga#28, scaffold)
- **Date**: 2026-05-09
- **Anchor**: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

## Context

TEEs have repeatedly broken (Plundervolt, ÆPIC, Downfall) and lock users to specific silicon.

## Decision

All chat security must hold without TEE assumptions; TEEs may opportunistically harden but never gate functionality.

## Consequences

Pros: portability, transparent threat model. Cons: server-side ML inference cannot run on encrypted inputs (acceptable — agents run client-side or in trusted infra).

## References

- [https://repello.ai/blog/owasp-llm-top-10-2026](https://repello.ai/blog/owasp-llm-top-10-2026)
- Design doc: [`/docs/chat/trinity-chat-design.md`](../chat/trinity-chat-design.md)
- Constitutional law: see [`crates/trios-chat/src/r_chat.rs`](../../crates/trios-chat/src/r_chat.rs)
