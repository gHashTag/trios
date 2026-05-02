# SR-02 — scarab-soul

Combines O-Type + Three-Type for soul-level agents.

## Term taxonomy

This ring carries two of SR-00's 16 `Term` variants:

- `Term::OType` — origin / monad
- `Term::ThreeType` — the soul-level binder

The wrapper struct `SoulScarab` exposes a 3-variant `SoulScarabType` enum
(`OType`, `ThreeType`, `Composite`) and pins both underlying terms
on every instance, so callers can pattern-match on either the variant
tag or the bound terms.

## Constitutional compliance

- R-RING-DEP-002: `serde` + SR-00 sibling only.
- I5: README.md, AGENTS.md, RING.md, Cargo.toml, src/lib.rs.
- L13: single-ring scope.
