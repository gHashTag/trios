# RING — trios-agent-memory (Gold Crate, GOLD IV)

## Identity

| Field    | Value |
|----------|-------|
| Metal    | 🥇 Gold |
| Type     | Crate (nested workspace) |
| Position | GOLD IV — anti-amnesia agent memory layer |
| Sealed   | No |

## Purpose

Bottom of the agent-memory dependency graph. SR-MEM-00 defines the
content-addressed `Triple`, `Provenance`, `AgentRole`, `MemoryKind`,
and `ForgetPolicy` types that every memory backend (KG, HDC, Neon
lessons store) must speak.

## Why GOLD IV is independent of GOLD I/II/III

- GOLD I  = pipeline (BPB, scarabs, queues, gardener)
- GOLD II = algorithm arena (entry hashes, env, theorems)
- GOLD III= outreach vocabulary
- GOLD IV = **agent memory** — what the agent remembers across runs

Separating the four lets SR-MEM-00..06 + BR-OUTPUT ship in parallel
with the other three branches.

## Ring Structure (L-ARCH-001)

```
crates/trios-agent-memory/
├── src/lib.rs                  ← re-export facade (≤ 50 LoC)
├── Cargo.toml                  ← outer GOLD, nested workspace
├── RING.md                     ← this file
└── rings/
    └── SR-MEM-00/              ← memory-types (day 1)
        ├── README.md
        ├── TASK.md
        ├── AGENTS.md
        ├── RING.md
        ├── Cargo.toml
        └── src/lib.rs
```

Future rings: SR-MEM-01 kg-client-adapter (#453), SR-MEM-05
episodic-bridge (#455), BR-OUTPUT AgentMemory assembler (#461).

## Anchor

`φ² + φ⁻² = 3` · TRINITY · GOLD IV
