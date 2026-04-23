# SEAL — trios-ext Reference Implementation v1

> **STATUS: 🔒 SEALED**
> Sealed by: TRINITY-SEAL agent + human approval
> Date: 2026-04-23
> Commit: 1768c3d663a663d25d29cd23ec3293f0a8b8085d

---

## What is sealed

`crates/trios-ext` is the **first reference implementation** of the Trinity Ring Architecture.
It is the canonical example of how a Gold Ring crate is structured.

## Ring structure (frozen)

```
crates/trios-ext/
├── src/                    ← 🥈 SILVER: Rust/WASM business logic
│   ├── lib.rs              ← WASM entry point (#[wasm_bindgen(start)])
│   ├── dom.rs              ← EX-01: DOM bridge, UI build
│   ├── mcp.rs              ← EX-02: MCP HTTP client
│   ├── bg.rs               ← EX-00: Background service worker logic
│   └── bridge/             ← SILVER sub-ring: types, comet transport
├── extension/              ← 🥉 BRONZE: Chrome Extension shell
│   ├── dist/               ← BR-EXT: compiled WASM artifacts (BUILD OUTPUT)
│   │   ├── trios_ext_br.js       ← wasm-bindgen JS glue
│   │   ├── trios_ext_br_bg.wasm  ← compiled WASM
│   │   ├── bg-sw.js              ← MV3 service worker
│   │   ├── bootstrap.js          ← sidepanel bootstrap
│   │   ├── github-bootstrap.js   ← GitHub injector
│   │   └── claude-bootstrap.js   ← Claude injector
│   ├── manifest.json       ← MV3 manifest
│   ├── sidepanel.html      ← side panel HTML shell
│   └── background.html     ← legacy MV2 (kept for reference)
├── xtask/                  ← 🥉 BRONZE: build helpers
├── Cargo.toml
├── SEAL.md                 ← THIS FILE
└── RING.md
```

## Laws governing this crate

- **L9**: All `dist/` references must point to existing files only
- **L10**: This seal cannot be broken without explicit human approval
- **L11**: No phantom imports — verify before referencing
- **R1–R5**: Ring isolation invariants

## What agents MAY do

- ✅ Read source files for reference
- ✅ Rebuild `dist/` artifacts via `wasm-pack build`
- ✅ Update Bronze ring HTML/manifest IF dist/ artifacts change

## What agents MAY NOT do

- ❌ Modify `src/*.rs` (Silver rings) without human approval
- ❌ Add new dependencies to `Cargo.toml` without human approval  
- ❌ Change the ring structure (add/remove/rename rings)
- ❌ Reference files in `dist/` without verifying they exist first
- ❌ Break the sealed reference chain:
  `manifest.json → sidepanel.html → dist/trios_ext_br.js → dist/trios_ext_br_bg.wasm ✅`

## Verified reference chain (v1)

```
manifest.json
├── side_panel → sidepanel.html
│   └── <script type="module"> → dist/trios_ext_br.js → dist/trios_ext_br_bg.wasm ✅
├── background.service_worker → dist/bg-sw.js ✅
├── content_scripts[github] → dist/trios_ext_br.js + dist/github-bootstrap.js ✅
├── content_scripts[claude] → dist/trios_ext_br.js + dist/claude-bootstrap.js ✅
└── web_accessible_resources → dist/trios_ext_br_bg.wasm ✅
```

---

*SEAL v1 — 2026-04-23 — trios Trinity Ring Architecture*
