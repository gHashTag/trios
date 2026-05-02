# AGENTS — BR-OUTPUT AgentMemory

**Ring:** BR-OUTPUT  
**Crate:** trios-agent-memory  
**Agent:** LEAD (Memory Maestro)  

## Laws for this ring

- L1: Rust only — no .sh, no wasm-pack
- L3: clippy -D warnings = 0
- L4: tests before merge
- L7: experience log written
- L8: push first, then PR
- L21: read-only forward from lessons (no direct write)
- I3: no business logic leaking into SR rings
- I5: README + TASK + AGENTS present (this file)
- R9: ring isolation — no sibling imports except via Cargo.toml

## Architecture note

BR-OUTPUT is the **assembly ring** — it imports from SR-MEM-00,
SR-MEM-01, SR-MEM-05 and re-exports the unified `AgentMemory` trait.
It does NOT contain algorithmic logic — that lives in the SR rings.

## Dependency graph

```
BR-OUTPUT (this ring)
  └── trios-agent-memory-sr-mem-00  (schema)
  └── trios-agent-memory-sr-mem-01  (kg-writer)
  └── trios-agent-memory-sr-mem-05  (episodic-bridge) ← #455
```

`phi² + phi⁻² = 3 · TRINITY 🌻`
