# AGENTS.md — SR-HACK-00 (trios-igla-race-hack)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: SR-HACK-00
- Package: `trios-igla-race-hack-sr-hack-00`
- Role: Vocabulary glossary (bottom of GOLD III dependency graph)
- Soul-name: `Vocab Vigilante`
- Codename: `LEAD`

## What this ring does

Defines `Term`, `Lane`, `Gate`, `RingTier`, and `all_terms()`.
No logic. No I/O. Pure data + display + serde.

## Rules (ABSOLUTE)

- R1  — Pure Rust only
- L6  — no async, no I/O, no subprocess, no network
- L13 — I-SCOPE: only this ring
- R-RING-DEP-002 — deps limited to `serde + serde_json` (dev)
- R-RING-FACADE-001 — outer crate `src/lib.rs` re-exports only

## You MAY

- ✅ Add new `Term` variants (non-breaking)
- ✅ Add new `Lane` variants
- ✅ Add new `Gate` variants
- ✅ Extend `RingTier::ColorVariant` with named tiers
- ✅ Add tests
- ✅ Improve `Display` strings IF you also bump a stable-format guard test

## You MAY NOT

- ❌ Import from any sibling ring (SR-HACK-01..05)
- ❌ Add I/O, filesystem, subprocess, network
- ❌ Add async / tokio
- ❌ Break existing Display strings without a guard-test bump (other rings depend on them)

## Build

```bash
cargo build  -p trios-igla-race-hack-sr-hack-00
cargo clippy -p trios-igla-race-hack-sr-hack-00 -- -D warnings
cargo test   -p trios-igla-race-hack-sr-hack-00 --no-deps
```

## Anchor

`φ² + φ⁻² = 3`
