# ADR-CHAT-007: Dual-LLM filter mandatory for tool calls

- **Status**: Accepted (Trinity Secure Chat EPIC trinity-fpga#28, scaffold)
- **Date**: 2026-05-09
- **Anchor**: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

## Context

Single-model planners are statistically vulnerable to prompt injection from untrusted RAG/web content.

## Decision

Planner LLM never sees raw tool output; a quarantined LLM summarises results into the trust domain.

## Consequences

Pros: empirical 60-90% reduction of injection success. Cons: 2× LLM cost; latency +200-500 ms.

## References

- [https://atlan.com/know/prompt-injection-attacks-ai-agents/](https://atlan.com/know/prompt-injection-attacks-ai-agents/)
- Design doc: [`/docs/chat/trinity-chat-design.md`](../chat/trinity-chat-design.md)
- Constitutional law: see [`crates/trios-chat/src/r_chat.rs`](../../crates/trios-chat/src/r_chat.rs)
