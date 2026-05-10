# ADR-CHAT-010: LXMF gateway-only (no on-by-default mesh-radio)

- **Status**: Accepted (Trinity Secure Chat EPIC trinity-fpga#28, scaffold)
- **Date**: 2026-05-09
- **Anchor**: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

## Context

LXMF/Reticulum is excellent for offline/disaster mode but not the default reliability target.

## Decision

Ship a gateway adapter (`trios-mesh-node` ↔ LXMF) but keep the default transport over QUIC + Tailscale Funnel.

## Consequences

Pros: optional resilience without affecting default UX. Cons: gateway is a small attack surface; isolated as a separate crate in a follow-up.

## References

- [https://github.com/markqvist/LXMF](https://github.com/markqvist/LXMF)
- Design doc: [`/docs/chat/trinity-chat-design.md`](../chat/trinity-chat-design.md)
- Constitutional law: see [`crates/trios-chat/src/r_chat.rs`](../../crates/trios-chat/src/r_chat.rs)
