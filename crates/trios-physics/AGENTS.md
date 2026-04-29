# AGENTS.md — trios-physics

> AAIF-compliant | MCP-compatible

- Crate: trios-physics (Gold)
- Repo: gHashTag/trios

## Ring map

| Ring | Package | Role |
|------|---------|------|
| PH-00 | trios-physics-ph00 | physical constants |
| PH-01 | trios-physics-ph01 | equations |
| BR-OUTPUT | trios-physics-br-output | assembly |

## Rules

- L-ARCH-001 future logic in rings/
- R9: no sibling imports
- L6: pure Rust
- Anchor: phi^2 + phi^-2 = 3
