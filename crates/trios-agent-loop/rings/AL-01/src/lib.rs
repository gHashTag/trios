//! AL-01 — Tool trait, ToolRegistry, builtin + browser tools.
//!
//! Ported from the retired TS agent core (`tools/framework.ts`,
//! `tools/tool-registry.ts`, `agent/tool-adapter.ts`). Browser tools speak
//! the BW-01 `BrowserCommand`/`BrowserResponse` contract from trios-browser
//! through a [`BrowserBridge`], so the live CDP driver can stay next to the
//! Chrome process (extension / A2A peer) exactly like in the TS design.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use trios_agent_loop_al00::ToolDef;
use trios_browser::proto::{BrowserCommand, BrowserResponse};

/// One agent tool: JSON-schema described, JSON-in / JSON-out.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    /// JSON Schema of the arguments object.
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, args: Value) -> Result<Value>;
}

/// Name-keyed tool registry (deterministic order for prompts and tests).
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Tool definitions in the AL-00 contract shape.
    pub fn defs(&self) -> Vec<ToolDef> {
        self.tools
            .values()
            .map(|t| ToolDef {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect()
    }

    pub async fn execute(&self, name: &str, args: Value) -> Result<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| anyhow!("unknown tool: {name}"))?;
        tool.execute(args).await
    }
}

// ---------------------------------------------------------------------------
// Builtin tools
// ---------------------------------------------------------------------------

/// `echo` — returns its input (loop plumbing smoke tool).
pub struct EchoTool;

#[async_trait::async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }
    fn description(&self) -> &str {
        "Echo the given text back. Use only when explicitly asked to echo."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"text": {"type": "string", "description": "Text to echo back"}},
            "required": ["text"]
        })
    }
    async fn execute(&self, args: Value) -> Result<Value> {
        let text = args
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("echo: `text` is required"))?;
        Ok(json!({ "text": text }))
    }
}

/// `time_now` — current UTC time (ISO 8601).
pub struct TimeNowTool;

#[async_trait::async_trait]
impl Tool for TimeNowTool {
    fn name(&self) -> &str {
        "time_now"
    }
    fn description(&self) -> &str {
        "Get the current date and time in UTC (ISO 8601)."
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }
    async fn execute(&self, _args: Value) -> Result<Value> {
        Ok(json!({ "utc": chrono::Utc::now().to_rfc3339() }))
    }
}

/// Register the builtin (non-browser) tools.
pub fn register_builtin_tools(registry: &mut ToolRegistry) {
    registry.register(Arc::new(EchoTool));
    registry.register(Arc::new(TimeNowTool));
}

// ---------------------------------------------------------------------------
// Browser tools (BW-01 contract via a bridge)
// ---------------------------------------------------------------------------

/// Transport that delivers a [`BrowserCommand`] to the live CDP driver
/// (extension over WS, A2A peer, or an in-process driver) and returns the
/// [`BrowserResponse`].
#[async_trait::async_trait]
pub trait BrowserBridge: Send + Sync {
    async fn execute(&self, command: BrowserCommand) -> Result<BrowserResponse>;
}

struct BrowserTool {
    bridge: Arc<dyn BrowserBridge>,
    name: &'static str,
    description: &'static str,
    schema: Value,
    build: fn(&Value) -> Result<BrowserCommand>,
}

#[async_trait::async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        self.description
    }
    fn parameters_schema(&self) -> Value {
        self.schema.clone()
    }
    async fn execute(&self, args: Value) -> Result<Value> {
        let command = (self.build)(&args)?;
        let response = self.bridge.execute(command).await?;
        serde_json::to_value(&response).map_err(Into::into)
    }
}

fn require_page(args: &Value) -> Result<i64> {
    args.get("page")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("`page` (integer page id) is required"))
}

/// Register the browser tools backed by `bridge`.
///
/// Tool set mirrors the BW-01 command surface: list_pages, goto, content,
/// screenshot, click, evaluate.
pub fn register_browser_tools(registry: &mut ToolRegistry, bridge: Arc<dyn BrowserBridge>) {
    let page_prop = json!({"type": "integer", "description": "Target page id (from browser_list_pages)"});
    let tools: Vec<BrowserTool> = vec![
        BrowserTool {
            bridge: bridge.clone(),
            name: "browser_list_pages",
            description: "List open browser pages (id, url, title).",
            schema: json!({"type": "object", "properties": {}}),
            build: |_| Ok(BrowserCommand::ListPages),
        },
        BrowserTool {
            bridge: bridge.clone(),
            name: "browser_goto",
            description: "Navigate a page to a URL.",
            schema: json!({
                "type": "object",
                "properties": {"page": page_prop, "url": {"type": "string"}},
                "required": ["page", "url"]
            }),
            build: |args| {
                let url = args
                    .get("url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("`url` is required"))?
                    .to_string();
                Ok(BrowserCommand::Goto { page: require_page(args)?, url })
            },
        },
        BrowserTool {
            bridge: bridge.clone(),
            name: "browser_content",
            description: "Get the text content of a page (optionally scoped to a CSS selector).",
            schema: json!({
                "type": "object",
                "properties": {"page": page_prop, "selector": {"type": "string"}},
                "required": ["page"]
            }),
            build: |args| {
                Ok(BrowserCommand::Content {
                    page: require_page(args)?,
                    selector: args.get("selector").and_then(Value::as_str).map(String::from),
                })
            },
        },
        BrowserTool {
            bridge: bridge.clone(),
            name: "browser_screenshot",
            description: "Take a screenshot of a page (base64).",
            schema: json!({
                "type": "object",
                "properties": {"page": page_prop, "full_page": {"type": "boolean"}},
                "required": ["page"]
            }),
            build: |args| {
                Ok(BrowserCommand::Screenshot {
                    page: require_page(args)?,
                    full_page: args.get("full_page").and_then(Value::as_bool).unwrap_or(false),
                })
            },
        },
        BrowserTool {
            bridge: bridge.clone(),
            name: "browser_click",
            description: "Click an element by CSS selector or snapshot node id.",
            schema: json!({
                "type": "object",
                "properties": {
                    "page": page_prop,
                    "selector": {"type": "string"},
                    "node_id": {"type": "integer"}
                },
                "required": ["page"]
            }),
            build: |args| {
                Ok(BrowserCommand::Click {
                    page: require_page(args)?,
                    selector: args.get("selector").and_then(Value::as_str).map(String::from),
                    node_id: args.get("node_id").and_then(Value::as_i64),
                })
            },
        },
        BrowserTool {
            bridge,
            name: "browser_evaluate",
            description: "Evaluate a JavaScript expression on a page and return the result.",
            schema: json!({
                "type": "object",
                "properties": {"page": page_prop, "expression": {"type": "string"}},
                "required": ["page", "expression"]
            }),
            build: |args| {
                let expression = args
                    .get("expression")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("`expression` is required"))?
                    .to_string();
                Ok(BrowserCommand::Evaluate { page: require_page(args)?, expression })
            },
        },
    ];
    for tool in tools {
        registry.register(Arc::new(tool));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[tokio::test]
    async fn registry_registers_and_dispatches() {
        let mut reg = ToolRegistry::new();
        register_builtin_tools(&mut reg);
        assert_eq!(reg.names(), vec!["echo", "time_now"]);
        let out = reg.execute("echo", json!({"text": "hi"})).await.unwrap();
        assert_eq!(out["text"], "hi");
        let now = reg.execute("time_now", json!({})).await.unwrap();
        assert!(now["utc"].as_str().unwrap().contains('T'));
    }

    #[tokio::test]
    async fn unknown_tool_is_an_error() {
        let reg = ToolRegistry::new();
        let err = reg.execute("nope", json!({})).await.unwrap_err();
        assert!(err.to_string().contains("unknown tool"));
    }

    #[tokio::test]
    async fn echo_requires_text() {
        let mut reg = ToolRegistry::new();
        register_builtin_tools(&mut reg);
        assert!(reg.execute("echo", json!({})).await.is_err());
    }

    #[test]
    fn defs_expose_json_schema() {
        let mut reg = ToolRegistry::new();
        register_builtin_tools(&mut reg);
        let defs = reg.defs();
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].name, "echo");
        assert_eq!(defs[0].parameters["required"][0], "text");
    }

    /// Bridge that records commands and answers with a canned response.
    struct RecordingBridge {
        seen: Mutex<Vec<BrowserCommand>>,
    }

    #[async_trait::async_trait]
    impl BrowserBridge for RecordingBridge {
        async fn execute(&self, command: BrowserCommand) -> Result<BrowserResponse> {
            self.seen.lock().unwrap().push(command);
            Ok(BrowserResponse::Ok)
        }
    }

    #[tokio::test]
    async fn browser_tools_map_args_to_commands() {
        let bridge = Arc::new(RecordingBridge { seen: Mutex::new(vec![]) });
        let mut reg = ToolRegistry::new();
        register_browser_tools(&mut reg, bridge.clone());
        assert_eq!(reg.len(), 6);

        reg.execute("browser_goto", json!({"page": 3, "url": "https://example.com"}))
            .await
            .unwrap();
        reg.execute("browser_content", json!({"page": 3, "selector": "h1"}))
            .await
            .unwrap();
        reg.execute("browser_click", json!({"page": 3, "node_id": 42}))
            .await
            .unwrap();

        let seen = bridge.seen.lock().unwrap();
        assert_eq!(
            seen[0],
            BrowserCommand::Goto { page: 3, url: "https://example.com".into() }
        );
        assert_eq!(
            seen[1],
            BrowserCommand::Content { page: 3, selector: Some("h1".into()) }
        );
        assert_eq!(
            seen[2],
            BrowserCommand::Click { page: 3, selector: None, node_id: Some(42) }
        );
    }

    #[tokio::test]
    async fn browser_goto_requires_url_and_page() {
        let bridge = Arc::new(RecordingBridge { seen: Mutex::new(vec![]) });
        let mut reg = ToolRegistry::new();
        register_browser_tools(&mut reg, bridge);
        assert!(reg.execute("browser_goto", json!({"page": 1})).await.is_err());
        assert!(reg
            .execute("browser_goto", json!({"url": "https://x.io"}))
            .await
            .is_err());
    }
}
