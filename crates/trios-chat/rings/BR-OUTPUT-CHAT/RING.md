# BR-OUTPUT-CHAT

**Tier:** Bronze (BR-OUTPUT — assembler / re-export)
**Owner:** Trinity Secure Chat
**Status:** [VERIFIED] re-exports compile-checked + smoke-tested

## Purpose
Single public surface that assembles the Trinity Secure Chat stack from
CR-CHAT-00..06 + CR-CHAT-LAWS. Downstream consumers (the `trios-chat`
shim crate, binaries, and external integrations) import only from
`trios_chat_br_output`.

## Wiring
- `identity` ← CR-CHAT-01
- `sealed`   ← CR-CHAT-01
- `ratchet`  ← CR-CHAT-02
- `group`    ← CR-CHAT-03
- `padding`  ← CR-CHAT-04
- `persist`  ← CR-CHAT-05 (trait)
- `capability` + `injection` ← CR-CHAT-06
- `r_chat`   ← CR-CHAT-LAWS

## Forbidden
- New logic, new types — re-exports only. Any new behaviour must land in
  a Silver CR-CHAT-* ring first.
- async / I/O — those belong to the sibling BR-IO-CHAT-05 ring.

## Mirrors
- Pattern: `trios-agent-memory/rings/BR-OUTPUT` (precedent set in #461).
