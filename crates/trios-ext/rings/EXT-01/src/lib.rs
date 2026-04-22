//! EXT-01 — Artifact Rendering
//!
//! Mirrors BR-OUTPUT types for WASM consumption.
//! Receives artifact JSON from the MCP server or z.ai API and renders
//! it as formatted HTML. Types mirror `trios-a2a-br-output` but are
//! lightweight (no std dependencies beyond wasm_bindgen).

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Artifact kind — mirrors `trios_a2a_br_output::ArtifactKind`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Code,
    Markdown,
    TestReport,
    BuildLog,
    Data,
    Diagram,
    Config,
    Custom(String),
}

impl std::fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactKind::Code => write!(f, "code"),
            ArtifactKind::Markdown => write!(f, "markdown"),
            ArtifactKind::TestReport => write!(f, "test_report"),
            ArtifactKind::BuildLog => write!(f, "build_log"),
            ArtifactKind::Data => write!(f, "data"),
            ArtifactKind::Diagram => write!(f, "diagram"),
            ArtifactKind::Config => write!(f, "config"),
            ArtifactKind::Custom(t) => write!(f, "custom:{t}"),
        }
    }
}

/// Artifact — mirrors `trios_a2a_br_output::Artifact`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub task_id: String,
    pub creator_did: String,
    pub kind: ArtifactKind,
    pub title: String,
    pub content: String,
    pub mime_type: String,
    pub extension: String,
    pub created_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Render an artifact as formatted HTML.
pub fn render_artifact_html(artifact: &Artifact) -> String {
    let escaped = html_escape(&artifact.content);
    let kind_badge = format!(
        "<span class=\"artifact-badge artifact-badge-{kind}\">{kind}</span>",
        kind = artifact.kind
    );

    let tags_html = if artifact.tags.is_empty() {
        String::new()
    } else {
        let tags: Vec<String> = artifact
            .tags
            .iter()
            .map(|t| format!("<span class=\"artifact-tag\">{t}</span>"))
            .collect();
        format!("<div class=\"artifact-tags\">{}</div>", tags.join(""))
    };

    let content_html = match &artifact.kind {
        ArtifactKind::Code => {
            format!(
                "<pre class=\"artifact-code\"><code>{escaped}</code></pre>"
            )
        }
        ArtifactKind::Markdown => {
            format!("<div class=\"artifact-markdown\">{escaped}</div>")
        }
        ArtifactKind::BuildLog => {
            format!("<pre class=\"artifact-log\">{escaped}</pre>")
        }
        _ => {
            format!("<pre class=\"artifact-data\">{escaped}</pre>")
        }
    };

    format!(
        "<div class=\"artifact-card\" data-id=\"{id}\">\
         <div class=\"artifact-header\">\
         <span class=\"artifact-title\">{title}</span>\
         {kind_badge}\
         </div>\
         <div class=\"artifact-meta\">\
         <span class=\"artifact-creator\">by {creator}</span>\
         <span class=\"artifact-time\">{time}</span>\
         </div>\
         {tags_html}\
         <div class=\"artifact-content\">{content_html}</div>\
         </div>",
        id = artifact.id,
        title = artifact.title,
        creator = artifact.creator_did,
        time = artifact.created_at,
    )
}

/// Render multiple artifacts as a combined HTML document.
pub fn render_artifacts_html(artifacts: &[Artifact]) -> String {
    let rendered: Vec<String> = artifacts.iter().map(render_artifact_html).collect();
    rendered.join("\n")
}

/// Parse artifacts from JSON (received from MCP server or z.ai).
#[wasm_bindgen]
pub fn parse_artifacts(json: &str) -> Result<JsValue, JsValue> {
    let artifacts: Vec<Artifact> =
        serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&artifacts)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Render artifacts JSON to HTML string.
#[wasm_bindgen]
pub fn render_artifacts(json: &str) -> Result<String, JsValue> {
    let artifacts: Vec<Artifact> =
        serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(render_artifacts_html(&artifacts))
}

/// Basic HTML escaping.
fn html_escape(s: &str) -> String {
    s.replace('&', "\x26amp;")
        .replace('<', "\x26lt;")
        .replace('>', "\x26gt;")
        .replace('"', "\x26quot;")
}

/// CSS for artifact rendering.
pub const ARTIFACT_CSS: &str = r#"
.artifact-card {
    background: var(--trios-bg-card, #1E1E32);
    border: 1px solid var(--trios-border, #2A2A45);
    border-radius: 8px;
    margin-bottom: 16px;
    overflow: hidden;
}
.artifact-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--trios-border, #2A2A45);
}
.artifact-title {
    font-weight: 600;
    color: var(--trios-primary, #D4A843);
}
.artifact-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 9999px;
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}
.artifact-badge-code { background: rgba(212,168,67,0.2); color: #D4A843; }
.artifact-badge-markdown { background: rgba(46,204,113,0.2); color: #2ECC71; }
.artifact-badge-test_report { background: rgba(243,156,18,0.2); color: #F39C12; }
.artifact-badge-build_log { background: rgba(231,76,60,0.2); color: #E74C3C; }
.artifact-badge-data { background: rgba(52,152,219,0.2); color: #3498DB; }
.artifact-meta {
    display: flex;
    gap: 16px;
    padding: 8px 16px;
    font-size: 12px;
    color: #8888A0;
}
.artifact-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 0 16px 8px;
}
.artifact-tag {
    display: inline-block;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 11px;
    background: rgba(212,168,67,0.1);
    color: #D4A843;
}
.artifact-content {
    padding: 16px;
}
.artifact-code, .artifact-log, .artifact-data {
    background: #0D0D1A;
    padding: 12px;
    border-radius: 4px;
    overflow-x: auto;
    font-family: monospace;
    font-size: 13px;
    line-height: 1.5;
    color: #E8E8F0;
}
.artifact-markdown {
    color: #E8E8F0;
    line-height: 1.6;
}
"#;
