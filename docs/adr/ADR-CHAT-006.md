# ADR-CHAT-006: Session-scoped capability tokens (≤1 h TTL)

- **Status**: Accepted (Trinity Secure Chat EPIC trinity-fpga#28, scaffold)
- **Date**: 2026-05-09
- **Anchor**: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

## Context

Standing OAuth-style scopes for agents allow lateral abuse if a session is hijacked.

## Decision

Capability tokens bind to (session_id, agent_id, ttl ≤ 3600 s) and require Issuer Ed25519 signature.

## Consequences

Pros: blast-radius bounded by 1 h. Cons: refresh round-trip every hour; cached refresh planned for L-CHAT-6.

## References

- [https://workos.com/blog/everything-your-team-needs-to-know-about-mcp-in-2026](https://workos.com/blog/everything-your-team-needs-to-know-about-mcp-in-2026)
- Design doc: [`/docs/chat/trinity-chat-design.md`](../chat/trinity-chat-design.md)
- Constitutional law: see [`crates/trios-chat/src/r_chat.rs`](../../crates/trios-chat/src/r_chat.rs)
