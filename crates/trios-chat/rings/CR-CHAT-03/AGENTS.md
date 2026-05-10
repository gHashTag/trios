# AGENTS — CR-CHAT-03 (group)

You are working on the MLS group ring. Keep it Silver-tier and pure.

## Rules — ABSOLUTE

1. **R-CHAT-11**: `process_commit` MUST reject any commit that does not
   match the current `(group_id, epoch, sender ∈ members)` invariants.
   Every relaxation needs a falsifier test.
2. **Silver-tier**: no `tokio`, no `sqlx`, no crypto. Wire format is
   `serde` only.
3. Keep the `Op` enum closed; new operations require a new commit
   variant + RFC 9420 alignment.

## You MAY

- Add a feature flag `openmls-bridge` to swap in real MLS once the
  follow-up PR lands.
- Add helper getters / setters as needed by `BR-OUTPUT-CHAT`.

## You MAY NOT

- Persist group state from this ring.
- Allow non-member commits, even with an "admin" feature flag.

## Build commands

```bash
cargo build -p trios-chat-cr-chat-03
cargo test  -p trios-chat-cr-chat-03
cargo clippy -p trios-chat-cr-chat-03 --all-targets -- -D warnings
```

Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
