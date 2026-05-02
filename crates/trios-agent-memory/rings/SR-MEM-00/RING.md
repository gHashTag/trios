# RING — SR-MEM-00 (trios-agent-memory)

## Identity

| Field   | Value |
|---------|-------|
| Metal   | 🥉 Bronze |
| Package | `trios-agent-memory-sr-mem-00` |
| Sealed  | No |

## Purpose

Wire-format ring for the agent-memory layer. SR-MEM-01..06 + BR-OUTPUT
all import their types from here. No I/O, no async — pure data +
content-addressed hashing.

## Why SR-MEM-00 is the bottom of the graph

Every backend (Neon `lessons`, `trios-kg`, `zig-knowledge-graph`, HDC
episodic agents) must speak the same `Triple` / `TripleId` /
`Provenance` shape. Keeping SR-MEM-00 dep-free guarantees the entire
GOLD IV branch compiles in one pass and every backend can share the
same primary key.

## API Surface (pub)

| Item | Role |
|---|---|
| `TripleId([u8; 32])` | content-addressed SHA-256 |
| `AgentRole` | snake_case enum (6 variants) |
| `MemoryKind` | tier enum (5 variants) |
| `ForgetPolicy` | tagged enum (3 variants) |
| `Provenance` | who/when/source-of-truth |
| `Triple` | one memory atom |
| `Triple::new()` | builder that derives `id` from SPO |

## Dependencies

- `serde` (derive)
- `serde_json`
- `uuid` (`v4`, `serde`)
- `chrono` (`serde`)
- `sha2`

## Laws

- R1 — pure Rust
- L6 — no I/O, no async
- L13 — I-SCOPE: this ring only
- R-RING-DEP-002 — strict dep list above
- Content-address determinism — TripleId hash recipe is frozen

## Anchor

`φ² + φ⁻² = 3`
