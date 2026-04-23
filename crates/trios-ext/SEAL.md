# 🔒 SEAL — trios-ext v1

**Sealed:** 2026-04-23
**Status:** REFERENCE IMPLEMENTATION — DO NOT MODIFY

## Reference Chain

```
manifest.json → sw.js → dist/trios_ext.js → trios_ext_bg.wasm
manifest.json → sidepanel.html → dist/trios_ext.js
sidepanel.html → styles/brand.css
```

## Sealed Files (SHA256)

| File | SHA256 |
|------|--------|
| `src/lib.rs` | `5504eda0aef390314fa9edab1db579db461675f0868684e891fb55739f15d46e` |
| `src/bg.rs` | `ead978d065933b6bfa9360eb75ad5e40c84f2223eedfcd9ef5e3adbcbac4fb6d` |
| `src/dom.rs` | `4918b61b03934a47decd260126970d1498178174e84ede7ff650ffb0afebbe10` |
| `src/mcp.rs` | `b98b86fb4f26b3312a4f4bb47b782d8297682569f176f8c11b12204bf03b45ba` |
| `src/bridge/mod.rs` | `79d1edf5f7a15144ebffd41aeef754b4e96b92182fbeb8ae8df34e083c6494b4` |
| `src/bridge/comet.rs` | `f94f1e1fdde25466899549738514af1df621febb55c51e4c163116feeb491d38` |
| `src/bridge/types.rs` | `703536227b3290aa563f504eeab9c7d2c3793a7cb9e3894a715cebffbe1cb7fa` |
| `src/bridge/tests.rs` | `93e212c9fd7561db6c7a5acefb3e69e531048f36ce759ad433940540a3f10f24` |

## Rules

- **READ-ONLY** for AI agents — no modifications to `src/` without human approval
- `extension/` (BR-EXT shell) may be updated only to fix reference chain
- Any SHA256 mismatch = seal broken = CI fails
- To unseal: update this file with new hashes + get @gHashTag approval
