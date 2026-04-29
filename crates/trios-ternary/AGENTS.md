# AGENTS.md — trios-ternary

> AAIF-compliant | MCP-compatible

## Identity

- Crate: trios-ternary
- Metal: Gold (Tier 1)
- Repo: gHashTag/trios

## Ring map

| Ring | Package | Role |
|------|---------|------|
| TR-00 | trios-ternary-tr00 | Trit, Tryte, balanced-ternary types |
| TR-01 | trios-ternary-tr01 | arithmetic ops |
| BR-OUTPUT | trios-ternary-br-output | assembly |

## Rules

- Read `LAWS.md` first
- L-ARCH-001: future logic in `rings/`
- R9: no sibling imports
- L6: pure Rust
- Anchor: `phi^2 + phi^-2 = 3`
