# AGENTS.md — trios-doctor

> AAIF-compliant (Linux Foundation Agentic AI Foundation)
> MCP-compatible agent instructions

## Identity

- Crate: trios-doctor
- Metal: Gold (first reference implementation of doctor pattern)
- Repo: gHashTag/trios

## What this crate does

Workspace health diagnostics and auto-repair. Runs cargo checks, reports results, optionally heals.

## Ring map

| Ring | Package | Role |
|------|---------|------|
| SILVER-RING-DR-00 | trios-doctor-dr00 | Types only |
| SILVER-RING-DR-01 | trios-doctor-dr01 | Check runner |
| SILVER-RING-DR-02 | trios-doctor-dr02 | Heal / auto-fix |
| SILVER-RING-DR-03 | trios-doctor-dr03 | Report / format |
| BRONZE-RING-DR | trios-doctor-bronze | CLI binary output |

## Rules (ABSOLUTE)

- Read `LAWS.md` before ANY action
- L-ARCH-001: Only `rings/` inside crate — NO `src/` at crate root
- R1–R5: Ring Isolation between rings
- L6: Pure Rust
- Each ring has its own AGENTS.md — read it before touching that ring

## You MAY NOT

- ❌ Create `src/` at `crates/trios-doctor/` level
- ❌ Add files outside `rings/`
- ❌ Cross-import Silver rings directly (only via Cargo.toml)
