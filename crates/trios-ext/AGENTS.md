# AGENTS.md — trios-ext (Crate Root)

> AAIF-compliant (Linux Foundation Agentic AI Foundation)
> MCP-compatible agent instructions

## Identity
- Crate: trios-ext
- Architecture: Trinity Ring Architecture (issue #247)

## Ring Layout

```
crates/trios-ext/
├── rings/
│   ├── SILVER-RING-EXT-00/   ← WASM entry point (depends on 01, 02, 03)
│   ├── SILVER-RING-EXT-01/   ← DOM bridge (standalone)
│   ├── SILVER-RING-EXT-02/   ← MCP client + BG (depends on 01)
│   ├── SILVER-RING-EXT-03/   ← Comet bridge / Types (depends on 01)
│   └── BRONZE-RING-EXT/      ← Chrome Extension output ring
├── AGENTS.md
├── README.md
└── SEAL.md
```

## Rules (ABSOLUTE)
- **L-ARCH-001**: `rings/` only — no `src/` at crate root
- **R1–R5**: Ring Isolation — each ring is a separate Cargo package
- **L6**: Pure Rust only
- **L9**: No handwritten JS — only dist/ artifacts from wasm-pack

## Build
```bash
# Build all rings
cargo build -p trios-ext-ring-ex00 -p trios-ext-ring-ex01 -p trios-ext-ring-ex02 -p trios-ext-ring-ex03

# Clippy all rings
cargo clippy -p trios-ext-ring-ex00 -p trios-ext-ring-ex01 -p trios-ext-ring-ex02 -p trios-ext-ring-ex03 -- -D warnings

# Test all rings
cargo test -p trios-ext-ring-ex00 -p trios-ext-ring-ex01 -p trios-ext-ring-ex02 -p trios-ext-ring-ex03
```
