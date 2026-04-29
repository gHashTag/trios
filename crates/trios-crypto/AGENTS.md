# AGENTS.md — trios-crypto

> AAIF-compliant | MCP-compatible

- Crate: trios-crypto (Gold)
- Repo: gHashTag/trios

## Ring map

| Ring | Package | Role |
|------|---------|------|
| CY-00 | trios-crypto-cy00 | identity types |
| CY-01 | trios-crypto-cy01 | signing |
| CY-02 | trios-crypto-cy02 | verification |
| BR-OUTPUT | trios-crypto-br-output | assembly |

## Rules

- L-ARCH-001 future logic in rings/
- R9: no sibling imports
- L6: pure Rust
- Anchor: phi^2 + phi^-2 = 3
