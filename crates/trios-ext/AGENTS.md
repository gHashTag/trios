# AGENTS.md — trios-ext

## Ring Architecture

| Ring | Crate | Purpose |
|------|-------|---------|
| EXT-00 | `trios-ext-00` | Shell & Transport (DOM + MCP) |
| EXT-01 | `trios-ext-01` | Artifact Rendering (BR-OUTPUT) |
| EXT-02 | `trios-ext-02` | Settings (chrome.storage.local) |
| EXT-03 | `trios-ext-03` | Content Injectors (GitHub, Claude.ai) |
| BR-EXT | `trios-ext-br` | WASM Entry Point |

## Agent Assignments

| Agent | Responsibility |
|-------|---------------|
| ECHO | Ring architecture, build system, documentation |
| ECHO | EXT-00: Sidepanel UI + MCP client |
| ECHO | EXT-01: Artifact rendering |
| ECHO | EXT-02: Settings persistence |
| ECHO | EXT-03: Content injectors |

## Invariants

### I5 — Ring Isolation
- Each ring is a separate crate with Cargo.toml, src/, README.md, TASK.md, AGENTS.md
- No circular dependencies between rings
- EXT-00 contains both DOM and MCP due to bidirectional coupling

### L9 — Zero Handwritten JS
- All logic in Rust → compiled to WASM
- Bootstrap loaders are the only JS files (I9 exception)

### BR-OUTPUT Parity
- EXT-01 types must mirror `trios-a2a-br-output` exactly

### Build
- `cargo check --target wasm32-unknown-unknown` for compile verification
- `cd rings/BR-EXT && wasm-pack build --target no-modules` for WASM build
- Raw strings for CSS: use `r#"..."#`
- HTML escaping: use `\x26amp;`, `\x26lt;`, `\x26gt;`, `\x26quot;`

## How to Extend

1. **New ring**: Create `rings/EXT-NN/` with Cargo.toml, src/lib.rs, README.md, TASK.md, AGENTS.md
2. **New tab**: Add to EXT-00 `build_ui()` tab-bar innerHTML + panel div
3. **New artifact kind**: Add variant to EXT-01 `ArtifactKind`, add CSS class
4. **New injector**: Add to EXT-03, register in manifest.json content_scripts
5. **New MCP method**: Add to EXT-00 `McpClient` impl and `handle_mcp_response()`
