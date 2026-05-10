# ADR-CHAT-005: Cover traffic opt-in (R-CHAT-10)

- **Status**: Accepted (Trinity Secure Chat EPIC trinity-fpga#28, scaffold)
- **Date**: 2026-05-09
- **Anchor**: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

## Context

Always-on cover traffic costs battery/bandwidth and is unacceptable as default for mobile.

## Decision

Cover traffic is OFF by default; enabled per-conversation behind a 'paranoid mode' flag.

## Consequences

Pros: clean energy profile out of the box. Cons: traffic-pattern adversaries unblocked unless flag is on; documented in threat model TM-7.

## References

- [https://simplex.chat/docs/simplex.html](https://simplex.chat/docs/simplex.html)
- Design doc: [`/docs/chat/trinity-chat-design.md`](../chat/trinity-chat-design.md)
- Constitutional law: see [`crates/trios-chat/src/r_chat.rs`](../../crates/trios-chat/src/r_chat.rs)
