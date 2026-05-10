# ADR-CHAT-002: Hybrid PQ KEM mandatory from day 1

- **Status**: Accepted (Trinity Secure Chat EPIC trinity-fpga#28, scaffold)
- **Date**: 2026-05-09
- **Anchor**: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

## Context

Harvest-now-decrypt-later threat from CRQC; classical X25519 alone is not sufficient by 2030.

## Decision

Every prekey bundle and ratchet DH step combines X25519 ⊕ ML-KEM-768 (Signal PQXDH pattern).

## Consequences

Pros: aligns with Signal’s PQXDH and the upcoming RingXKEM upgrade. Cons: +1184 B per bundle, +slower KEM than DH.

## References

- [https://gniot.fr/assets/slides/2025/2025-12-signal.pdf](https://gniot.fr/assets/slides/2025/2025-12-signal.pdf)
- Design doc: [`/docs/chat/trinity-chat-design.md`](../chat/trinity-chat-design.md)
- Constitutional law: see [`crates/trios-chat/src/r_chat.rs`](../../crates/trios-chat/src/r_chat.rs)
