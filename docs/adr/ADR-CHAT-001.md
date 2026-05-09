# ADR-CHAT-001: MLS over n-pairwise for groups (RFC 9420)

- **Status**: Accepted (Trinity Secure Chat EPIC trinity-fpga#28, scaffold)
- **Date**: 2026-05-09
- **Anchor**: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

## Context

Need forward-secure, post-compromise-secure group chat for users + agent bots.

## Decision

Adopt MLS RFC 9420 for groups ≥ 3 members; n-pairwise Signal sessions only for 2-party.

## Consequences

Pros: O(log N) key updates, formal security proofs, IETF standard. Cons: ratchet tree complexity, GroupKeyPackage distribution required.

## References

- [https://datatracker.ietf.org/doc/rfc9420/](https://datatracker.ietf.org/doc/rfc9420/)
- Design doc: [`/docs/chat/trinity-chat-design.md`](../chat/trinity-chat-design.md)
- Constitutional law: see [`crates/trios-chat/src/r_chat.rs`](../../crates/trios-chat/src/r_chat.rs)
