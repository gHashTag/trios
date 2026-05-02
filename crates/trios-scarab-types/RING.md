# trios-scarab-types — parallel-execution foundation

| Field | Value |
|---|---|
| Type | Crate (inner workspace) |
| Issue | [#479](https://github.com/gHashTag/trios/issues/479) |
| EPIC | [#446](https://github.com/gHashTag/trios/issues/446) |
| Soul | `Constitutional-Cartographer` |
| Codename | `LEAD` (origin); ring-pattern foundation for parallel agent dispatch |

## Ring layout

```
crates/trios-scarab-types/
├── src/lib.rs                ← facade, re-exports only (12 LoC)
├── Cargo.toml                ← inner workspace member list
└── rings/
    ├── SR-00/  scarab-types  ← 16-variant Term enum + TermScarab
    ├── SR-01/  scarab-lane   ← O-Type + Two-Type   → LaneScarab
    ├── SR-02/  scarab-soul   ← O-Type + Three-Type → SoulScarab
    ├── SR-03/  scarab-gate   ← O-Type + Four-Type  → GateScarab(Four)
    └── SR-04/  scarab-gate   ← O-Type + Five-Type  → GateScarab(Five)
```

## Constitutional compliance

- **R-RING-FACADE-001** ✓ outer `src/lib.rs` is 12 LoC, re-exports only.
- **R-RING-DEP-002** ✓ rings depend on `serde` + the SR-00 sibling only.
- **R-L6-PURE-007** ✓ no `.py` here.
- **L13** ✓ each ring confines itself to its own folder.
- **I5** ✓ every ring ships README.md, AGENTS.md, RING.md, Cargo.toml,
  src/lib.rs.

## Tests — 29 GREEN

```
SR-00: 5 (16-variant alphabet, slugs, Display, markdown, serde)
SR-01: 6 (Lane scarab + LaneScarabType)
SR-02: 6 (Soul scarab + SoulScarabType)
SR-03: 6 (Gate-four scarab + GateScarabFourType)
SR-04: 6 (Gate-five scarab + GateScarabFiveType)
```

`cargo clippy --workspace --all-targets -- -D warnings` → **0 warnings**.

## Why this exists

Per #479: enables 5 parallel agents to start simultaneously via
`trinity-bootstrap --codename` (PR #469). Each codename binds to one
SR ring and one virtual branch, so anti-collision is type-enforced at
compile time rather than relying on shared mutex state.
