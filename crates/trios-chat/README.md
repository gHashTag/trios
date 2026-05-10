# `trios-chat` — Trinity Secure Chat

> Privacy-first chat between users and agent bots over `trios-mesh-node`.
>
> Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
>
> Parent EPIC: [trinity-fpga#28](https://github.com/gHashTag/trinity-fpga/issues/28)
> Builds on:   [trinity-fpga#22](https://github.com/gHashTag/trinity-fpga/issues/22) ✅ + [trios#629](https://github.com/gHashTag/trios/pull/629) ✅

This crate is the EPIC #28 scaffold. It ships a working subset of the protocol
(identity, ratchet skeleton, sealed envelope, padding, capability tokens,
prompt-injection filter, R-CHAT-1..12 laws constant) plus the test harness
(25 e2e tests, 200-attack falsifier corpus, 7 Coq invariants).

## Status (R5 honesty tags)

| Module        | Lane       | Status          |
|---------------|------------|-----------------|
| `identity`    | L-CHAT-1   | `[VERIFIED]` Ed25519+X25519 · `[ASPIRATIONAL]` ML-KEM placeholder |
| `ratchet`     | L-CHAT-2   | `[ASPIRATIONAL]` skeleton only — full Triple Ratchet in follow-up |
| `sealed`      | L-CHAT-4   | `[VERIFIED]` round-trip + tamper rejection |
| `capability`  | L-CHAT-6   | `[VERIFIED]` issue/verify/scope/ttl |
| `injection`   | L-CHAT-6   | `[VERIFIED]` deny-list pre-screen + dual-LLM hooks |
| `padding`     | L-CHAT-7   | `[VERIFIED]` 4 fixed classes |
| `r_chat`      | LAWS       | `[VERIFIED]` 12 constitutional laws |
| Coq stubs     | L-CHAT-9   | 6 `Defined`, 1 `Admitted` (budget per R5) |
| 200-attack corpus | L-CHAT-10 | direct 100 % · indirect 90 % · multi-turn 100 % · capability_abuse 10 % (deny-list only) |

## Lanes (10 sub-issues)

| # | Lane       | Issue |
|---|------------|-------|
| 1 | Identity & Onboarding         | [#29](https://github.com/gHashTag/trinity-fpga/issues/29) |
| 2 | Triple Ratchet                | [#30](https://github.com/gHashTag/trinity-fpga/issues/30) |
| 3 | MLS group                     | [#31](https://github.com/gHashTag/trinity-fpga/issues/31) |
| 4 | Sealed Sender                 | [#32](https://github.com/gHashTag/trinity-fpga/issues/32) |
| 5 | Persistence                   | [#33](https://github.com/gHashTag/trinity-fpga/issues/33) |
| 6 | Agent capability + dual-LLM   | [#34](https://github.com/gHashTag/trinity-fpga/issues/34) |
| 7 | Anti-metadata                 | [#35](https://github.com/gHashTag/trinity-fpga/issues/35) |
| 8 | PQ migration                  | [#36](https://github.com/gHashTag/trinity-fpga/issues/36) |
| 9 | Coq invariants                | [#37](https://github.com/gHashTag/trinity-fpga/issues/37) |
|10 | e2e_chat + falsifier corpus   | [#38](https://github.com/gHashTag/trinity-fpga/issues/38) |

## Constitutional laws — R-CHAT-1..R-CHAT-12

See [`src/r_chat.rs`](src/r_chat.rs). Removing or modifying any law fails CI.

## Quick start

```bash
cargo test -p trios-chat --lib            # 35/35 unit tests
cargo run  -p trios-chat --bin e2e_chat_25     # 25/25 e2e tests
cargo run  -p trios-chat --bin falsifier_runner # 200-attack corpus
```

## Design doc

Full design (29 KB, 21 sources, 14-param × 9-competitor matrix, 6-week roadmap,
10 ADRs) lives at [`/docs/chat/trinity-chat-design.md`](../../docs/chat/trinity-chat-design.md).

## ADRs

[`/docs/adr/ADR-CHAT-001..010`](../../docs/adr/) — see each file for context,
decision, consequences. Highlights:

- **001** MLS over n-pairwise (RFC 9420) — picked for forward-secure groups.
- **002** Hybrid PQ from day 1 — Signal PQXDH + RingXKEM.
- **004** Fixed padding classes {256, 1024, 4096, 16384} — R-CHAT-9.
- **007** Dual-LLM filter mandatory — R-CHAT-7.

## Citations

Design and ADRs cite 21 primary sources (Signal PQXDH 2026, RFC 9420,
Partial-MLS draft, MCP Auth 2026, OWASP LLM Top-10 2026, SimpleX, LXMF,
A2A, deniability paper, …). Full list in
[`/docs/chat/trinity-chat-design.md`](../../docs/chat/trinity-chat-design.md).
