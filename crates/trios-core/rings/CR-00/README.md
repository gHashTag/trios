# CR-00 — Core Constants & Identity

## Purpose
Foundational types and constants for the trios ecosystem. All other crates depend on this ring.

## API
- `TriosVersion` — semantic version struct
- `NodeId` — unique node identity type
- Core error types: `TriosError`, `TriosResult<T>`

## Dependencies
None (ring 00 = no upstream deps)

## Invariants
- Zero dependencies outside std
- All types: `Debug + Clone + PartialEq`
- `cargo check --target wasm32-unknown-unknown` passes
