# EXT-01 — Artifact Rendering

Mirrors BR-OUTPUT types for WASM consumption. Receives artifact JSON and renders formatted HTML.

## API
- `ArtifactKind` — Code, Markdown, TestReport, BuildLog, Data, Diagram, Config, Custom
- `Artifact` — Full artifact struct (id, task_id, creator_did, kind, title, content, tags)
- `render_artifact_html(artifact)` → HTML string
- `render_artifacts_html(artifacts)` → combined HTML
- `render_artifacts(json)` — wasm_bindgen export (JSON → HTML)
- `parse_artifacts(json)` — wasm_bindgen export (JSON → JsValue)
- `ARTIFACT_CSS` — Scoped CSS for artifact cards

## Dependencies
None (standalone ring).

## Usage
```rust
use trios_ext_01::{render_artifacts, ArtifactKind, ARTIFACT_CSS};

let html = render_artifacts(r#"[{"id":"1","kind":"code",...}]"#)?;
```
