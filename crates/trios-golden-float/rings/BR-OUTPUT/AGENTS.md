# AGENTS.md — BR-OUTPUT (trios-golden-float)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: BR-OUTPUT
- Package: trios-golden-float-br-output
- Role: assembly + future router

## Rules (ABSOLUTE)

- May import from GF-00, GF-01
- R9: No sibling imports (none exist at this level)
- L6: Pure Rust only

## You MAY

- ✅ Add MCP tool definitions
- ✅ Add router/dispatch logic
- ✅ Re-export ring surfaces

## You MAY NOT

- ❌ Add business logic that belongs in GF-00 or GF-01
- ❌ Add async runtime without approval
- ❌ Break the anchor `phi^2 + phi^-2 = 3`
