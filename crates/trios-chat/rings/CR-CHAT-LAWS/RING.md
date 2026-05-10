# CR-CHAT-LAWS

**Tier:** Silver (CR-* — Core Rule)
**Owner:** Trinity Secure Chat
**Status:** [VERIFIED] 4/4 tests pass

## Purpose
Pure constants ring exporting the 12 constitutional laws (R-CHAT-1..12) and the SHA-256 commitment over them. Single source of truth for the chat constitution. Any other ring that needs to assert constitutional adherence MUST depend on this ring (no copies allowed).

## Inputs
None — pure constants.

## Outputs
- `R_CHAT_LAWS: [&str; 12]`
- `laws_hash() -> [u8; 32]`

## Invariants
- `R_CHAT_LAWS.len() == 12`
- `laws_hash()` deterministic
- Each law starts with the canonical prefix `R-CHAT-N`
- All laws unique

## Deps
- `trios-chat-cr-chat-00` (errors)
- `sha2`

## Forbidden
- async / I/O / randomness / network — Silver tier rules.
