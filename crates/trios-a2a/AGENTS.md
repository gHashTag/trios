# Agent Instructions — trios-a2a

## Context
trios-a2a is the REFERENCE IMPLEMENTATION of ring architecture.
Do NOT modify without explicit instruction. Read it to understand the pattern.

## Ring structure (8 SR-rings + BR-OUTPUT)
SR-00: Identity, SR-01: Registry, SR-02: Protocol, SR-03: Transport,
SR-04: Dispatch, SR-05: Tasks, SR-06: Messages, SR-07: Events, BR-OUTPUT

## Forbidden
- Do not rename rings
- Do not change public API without updating all dependents
- Do not add .sh files

## Verification
```bash
cargo test -p trios-a2a
cargo clippy -p trios-a2a -- -D warnings
```
