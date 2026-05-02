# SR-01 — scarab-lane

Combines O-Type + Two-Type for lane-level agents.

## Term taxonomy

This ring carries two of SR-00's 16 `Term` variants:

- `Term::OType` — origin / monad
- `Term::TwoType` — the lane-level binder

The wrapper struct `LaneScarab` exposes a 3-variant `LaneScarabType` enum
(`OType`, `TwoType`, `Composite`) and pins both underlying terms
on every instance, so callers can pattern-match on either the variant
tag or the bound terms.

## Constitutional compliance

- R-RING-DEP-002: `serde` + SR-00 sibling only.
- I5: README.md, AGENTS.md, RING.md, Cargo.toml, src/lib.rs.
- L13: single-ring scope.
