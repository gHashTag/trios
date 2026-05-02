# BR-OUTPUT — AgentMemory Assembler

**Ring:** BR-OUTPUT  
**Crate:** `trios-agent-memory`  
**Tier:** GOLD IV  
**Kingdom:** Rust  
**Soul-name:** Memory Maestro  

## Purpose

Public assembler that exposes the four-verb `AgentMemory` trait:
`recall`, `remember`, `reflect`, `forget`.

Every agent (scarab, gardener, trainer, doctor, claude, lead) consumes
this single trait via the default implementation `KgAgentMemory`.

## Dependency chain

```
BR-OUTPUT
  ├── SR-MEM-00  (KG schema + RDF types)
  ├── SR-MEM-01  (KG writer / reader)
  └── SR-MEM-05  (episodic bridge — lessons + HDC)
```

## Anchor

`phi² + phi⁻² = 3 · TRINITY · NEVER STOP 🌻`

## Status

- [x] I5 trinity scaffolded (README + TASK + AGENTS)
- [x] Trait skeleton + `KgAgentMemory` stub
- [ ] Real wiring to SR-MEM-01 / SR-MEM-05 (after #455 merged)
- [ ] Integration test: remember → recall SHA-256-equal triples
- [ ] Stubs SR-MEM-02/03/04/06 TODO markers

Closes #461
