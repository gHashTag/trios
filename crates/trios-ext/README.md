# 🔧 trios-ext — Chrome Extension (Ring Isolation Architecture)

Chrome Extension built entirely in Rust → WASM. Zero handwritten JavaScript (L9 compliance).

## Ring Architecture

```
trios-ext/
├── Cargo.toml              ← Workspace root ([package] + [workspace])
├── src/lib.rs              ← Re-exports from rings (backward compat)
├── rings/
│   ├── EXT-00/             ← Shell & Transport (DOM + MCP)
│   ├── EXT-01/             ← Artifact Rendering (BR-OUTPUT)
│   ├── EXT-02/             ← Settings (chrome.storage.local)
│   ├── EXT-03/             ← Content Injectors (GitHub, Claude.ai)
│   └── BR-EXT/             ← WASM Entry Point (wires all rings)
├── extension/              ← Chrome MV3 extension assets
├── ring-silver/            ← Legacy build ring
├── ring-bronze/            ← Legacy deploy ring
├── README.md
├── TASK.md
└── AGENTS.md
```

## Dependency Graph

```
BR-EXT ──→ EXT-00 (Shell & Transport)
        ──→ EXT-01 (Artifact Rendering)
        ──→ EXT-02 (Settings)
        ──→ EXT-03 (Content Injectors)

EXT-00 ──→ EXT-01 (artifact CSS + rendering)
        ──→ EXT-02 (settings API key)
EXT-03 ──→ EXT-00 (DOM document())
```

No circular dependencies.

## Sidepanel Tabs

| Tab | Ring | Description |
|-----|------|-------------|
| Chat | EXT-00 | MCP/z.ai direct chat |
| Agents | EXT-00 | Agent list |
| Tools | EXT-00 | MCP tools browser |
| Issues | EXT-00 | GitHub issue tracker |
| Artifacts | EXT-00+01 | BR-OUTPUT artifact viewer |
| ⚙ Settings | EXT-00+02 | API key management |

## Build

```bash
# Check all rings:
cargo check --target wasm32-unknown-unknown

# Build WASM binary:
cd crates/trios-ext/rings/BR-EXT
wasm-pack build --target no-modules --out-dir pkg
```

## Load Extension

1. Build WASM package (see above)
2. Open `chrome://extensions`
3. Enable Developer mode
4. Load unpacked → select `crates/trios-ext/ring-bronze/extension/`

## Invariants

- **I5**: Ring isolation — each ring is a separate crate
- **L9**: Zero handwritten JS (bootstrap loaders excepted per I9)
- **BR-OUTPUT parity**: EXT-01 types mirror `trios-a2a-br-output`
