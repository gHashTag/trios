# AGENTS.md — EXT-01

## Invariants
- Types must mirror `trios-a2a-br-output` exactly
- No dependencies on other EXT rings
- HTML escaping uses `\x26` hex escapes to avoid raw string conflicts

## Testing
```bash
cargo check --target wasm32-unknown-unknown
```

## How to Extend
- New artifact kind: Add variant to `ArtifactKind`, add CSS class `.artifact-badge-{kind}`
- New rendering mode: Add match arm in `render_artifact_html()`
