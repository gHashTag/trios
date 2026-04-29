# AGENTS.md — trios-hdc

> AAIF-compliant | MCP-compatible

- Crate: trios-hdc (Gold)
- Repo: gHashTag/trios

## Ring map

| Ring | Package | Role |
|------|---------|------|
| HD-00 | trios-hdc-hd00 | Hypervector |
| HD-01 | trios-hdc-hd01 | bind, bundle, similarity |
| BR-OUTPUT | trios-hdc-br-output | assembly |

## Rules

- L-ARCH-001 future logic in rings/
- R9: no sibling imports
- L6: pure Rust
- Anchor: phi^2 + phi^-2 = 3
