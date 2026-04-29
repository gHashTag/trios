# AGENTS.md — trios-vsa

> AAIF-compliant | MCP-compatible

- Crate: trios-vsa (Gold)
- Repo: gHashTag/trios

## Ring map

| Ring | Package | Role |
|------|---------|------|
| VS-00 | trios-vsa-vs00 | Symbol algebra |
| VS-01 | trios-vsa-vs01 | bind/unbind/superpose |
| BR-OUTPUT | trios-vsa-br-output | assembly |

## Rules

- L-ARCH-001 future logic in rings/
- R9: no sibling imports
- L6: pure Rust
- Anchor: phi^2 + phi^-2 = 3
