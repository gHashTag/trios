# AGENTS.md — trios-golden-float

> AAIF-compliant (Linux Foundation Agentic AI Foundation)
> MCP-compatible agent instructions

## Identity

- Crate: trios-golden-float
- Metal: Gold (Tier 1)
- Repo: gHashTag/trios

## What this crate does

Golden-ratio numeric core: GF16 newtype, phi constants, arithmetic ops
that preserve the anchor `phi^2 + phi^-2 = 3`.

## Ring map

| Ring | Package | Role | Sealed |
|------|---------|------|--------|
| GF-00 | trios-golden-float-gf00 | phi constants, GF16 type | No |
| GF-01 | trios-golden-float-gf01 | arithmetic operations | No |
| BR-OUTPUT | trios-golden-float-br-output | assembly + router | No |

## Rules (ABSOLUTE)

- Read `LAWS.md` before ANY action
- L-ARCH-001: Future logic lives in `rings/` — `src/lib.rs` will become re-export only
- R1–R5: Ring Isolation — no cross-imports except via Cargo.toml
- R9: Rings cannot import siblings at the same level
- L6: Pure Rust only
- Each ring has its own `AGENTS.md` — read it before touching that ring

## You MAY NOT

- ❌ Add business logic to `src/lib.rs` going forward (legacy preserved)
- ❌ Cross-import rings without Cargo.toml declaration
- ❌ Break the anchor `phi^2 + phi^-2 = 3`
