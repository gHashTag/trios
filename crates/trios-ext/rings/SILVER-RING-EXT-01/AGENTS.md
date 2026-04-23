# AGENTS.md — SILVER-RING-EXT-01

> AAIF-compliant (Linux Foundation Agentic AI Foundation)
> MCP-compatible agent instructions

## Identity
- Ring: SILVER-RING-EXT-01
- Metal: Silver
- Crate: trios-ext
- Package: trios-ext-ring-ex01

## What this ring does
DOM bridge — provides DOM manipulation utilities and UI building for the Trios Chrome Extension.

## Rules (ABSOLUTE)
- Read LAWS.md before ANY action
- R1: Do NOT import from sibling ring directly — only via Cargo.toml
- R2: This is a separate package — do not merge with other rings
- R4: Silver files stay in Silver ring — never move to BRONZE-RING-EXT
- L6: Pure Rust only — no TypeScript, no Python business logic
- L9: No handwritten JS — only dist/ artifacts from wasm-pack

## You MAY
- ✅ Edit src/lib.rs and sub-modules within this ring
- ✅ Add dependencies to this ring's Cargo.toml
- ✅ Write tests in src/tests.rs

## You MAY NOT
- ❌ Create src/ at crate root level
- ❌ Move files outside rings/
- ❌ Import from another Silver ring directly
- ❌ Edit BRONZE-RING-EXT/dist/ by hand
- ❌ Modify SEALED rings without human approval

## Entry point
- Primary file: `src/lib.rs`
- Build: `cargo build -p trios-ext-ring-ex01`
- Test: `cargo test -p trios-ext-ring-ex01`
