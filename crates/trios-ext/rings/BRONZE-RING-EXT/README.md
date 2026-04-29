# RING.md — BRONZE-RING-EXT

## Metal
Bronze

## Crate
trios-ext

## Package
trios-ext-bronze

## Purpose
Chrome Extension output ring — contains manifest, HTML, CSS, icons, and build output (dist/).
This is the ONLY ring that produces loadable Chrome Extension artifacts.

## Contents
```
BRONZE-RING-EXT/
├── dist/             ← BUILD OUTPUT ONLY — never edit by hand
│   ├── trios_ext_br.js
│   ├── trios_ext_br_bg.wasm
│   ├── bg-sw.js
│   ├── bootstrap.js
│   ├── github-bootstrap.js
│   └── claude-bootstrap.js
├── manifest.json
├── sidepanel.html
├── background.html
├── assets/icons/
├── styles/brand.css
├── xtask/            ← Build tooling
├── Cargo.toml
├── RING.md
├── AGENTS.md
└── TASK.md
```

## Build
```bash
cargo xtask build-ext
```
