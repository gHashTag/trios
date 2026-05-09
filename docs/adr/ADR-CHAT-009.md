# ADR-CHAT-009: RingXKEM migration on Day 90

- **Status**: Accepted (Trinity Secure Chat EPIC trinity-fpga#28, scaffold)
- **Date**: 2026-05-09
- **Anchor**: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

## Context

Signal’s 2025-12 announcement upgrades PQXDH → RingXKEM for ratchet PCS; we want parity.

## Decision

Schedule a Day-90 migration after launch; protocol_version bump to 2 with compat shim.

## Consequences

Pros: stays on the Signal frontier. Cons: forces clients to rotate prekeys; coordinated via Trinity registry.

## References

- [https://gniot.fr/assets/slides/2025/2025-12-signal.pdf](https://gniot.fr/assets/slides/2025/2025-12-signal.pdf)
- Design doc: [`/docs/chat/trinity-chat-design.md`](../chat/trinity-chat-design.md)
- Constitutional law: see [`crates/trios-chat/src/r_chat.rs`](../../crates/trios-chat/src/r_chat.rs)
