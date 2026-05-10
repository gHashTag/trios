# AGENTS — CR-CHAT-07 (anti-correlation)

## Identity

You are working on **CR-CHAT-07** — the wire-time privacy ring of
`trios-chat`.

## What this ring does

Implements `CoverScheduler` (decides Real vs. Cover emission per tick)
and `uniform_gap_ms` (quantises measured inter-envelope gaps into one of
four canonical classes). Implements **R-CHAT-10**.

## Rules — ABSOLUTE

1. **Silver-tier**. No async, no I/O, no crypto, no randomness.
2. **Only depends on CR-CHAT-00**. No other ring deps allowed.
3. **R-CHAT-10 is law**. The wire observer must NOT be able to
   distinguish a Real emission from a Cover emission via the public
   API.
4. **Determinism**. `CoverScheduler` is deterministic given the call
   sequence — never sample from system time, never use randomness here.
5. **Falsifier-first**. Every code path needs a unit test that proves
   it rejects malformed input or attests the wire-indistinguishability
   property.

## You MAY

- Add helper predicates (e.g. `is_canonical_gap(g: u64) -> bool`).
- Generalise the scheduler to weighted cadences as long as the
  determinism property holds.
- Extend `CANONICAL_GAPS_MS` only with documented motivation in
  `RING.md`.

## You MAY NOT

- Pull in `tokio`, `sqlx`, `sea-orm`, `reqwest`, `chacha20poly1305`, or
  any randomness crate.
- Re-export anything from CR-CHAT-01 / 02 / 03 / 04 / 05 / 06 — flow
  goes the other way (BR-IO-CHAT-07 will consume this ring).

## Build commands

```bash
cargo build -p trios-chat-cr-chat-07
cargo test  -p trios-chat-cr-chat-07
cargo clippy -p trios-chat-cr-chat-07 --all-targets -- -D warnings
```

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · UNLINKABLE`
