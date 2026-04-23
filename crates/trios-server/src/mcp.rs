use rust_mcp_schema::{
    CallToolRequestParams, CallToolResult, ContentBlock, ListToolsResult, TextContent,
    Tool, ToolInputSchema,
};
use serde_json::{json, Value};
use std::sync::LazyLock;
use tracing::info;

use crate::tools;

static TOOL_DEFINITIONS: LazyLock<Vec<Tool>> = LazyLock::new(build_tool_definitions);

fn prop_type(type_str: &str) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    m.insert("type".into(), Value::String(type_str.into()));
    m
}

fn prop_array_items() -> serde_json::Map<String, Value> {
    json!({"type": "array", "items": {"type": "string"}})
        .as_object()
        .unwrap()
        .clone()
}

fn build_tool_definitions() -> Vec<Tool> {
    fn make_tool(
        name: &str,
        description: &str,
        required: Vec<&str>,
        properties: Vec<(&str, serde_json::Map<String, Value>)>,
    ) -> Tool {
        Tool {
            name: name.into(),
            description: Some(description.into()),
            input_schema: ToolInputSchema::new(
                required.into_iter().map(String::from).collect(),
                Some(
                    properties
                        .into_iter()
                        .map(|(k, v)| (k.into(), v))
                        .collect(),
                ),
                None,
            ),
            annotations: None,
            meta: None,
            icons: vec![],
            execution: None,
            output_schema: None,
            title: None,
        }
    }

    vec![
        // === Git Basic (5) ===
        make_tool(
            "git_status",
            "List all changed files in a repository",
            vec!["repo_path"],
            vec![("repo_path", prop_type("string"))],
        ),
        make_tool(
            "git_stage_files",
            "Stage specific files for commit",
            vec!["repo_path", "paths"],
            vec![
                ("repo_path", prop_type("string")),
                ("paths", prop_array_items()),
            ],
        ),
        make_tool(
            "git_unstage_files",
            "Unstage specific files",
            vec!["repo_path", "paths"],
            vec![
                ("repo_path", prop_type("string")),
                ("paths", prop_array_items()),
            ],
        ),
        make_tool(
            "git_commit",
            "Commit all staged files",
            vec!["repo_path", "message"],
            vec![
                ("repo_path", prop_type("string")),
                ("message", prop_type("string")),
            ],
        ),
        make_tool(
            "git_create_branch",
            "Create a new branch from HEAD",
            vec!["repo_path", "name"],
            vec![
                ("repo_path", prop_type("string")),
                ("name", prop_type("string")),
            ],
        ),
        // === Git Extended (4) ===
        make_tool(
            "git_log",
            "Get commit history",
            vec!["repo_path"],
            vec![
                ("repo_path", prop_type("string")),
                ("limit", json!({"type": "integer", "default": 10}).as_object().unwrap().clone()),
            ],
        ),
        make_tool(
            "git_diff",
            "Show unstaged changes (optionally filtered by file)",
            vec!["repo_path"],
            vec![
                ("repo_path", prop_type("string")),
                ("file", prop_type("string")),
            ],
        ),
        make_tool(
            "git_stash",
            "Stash current working directory changes",
            vec!["repo_path"],
            vec![("repo_path", prop_type("string"))],
        ),
        make_tool(
            "git_checkout",
            "Switch to an existing branch",
            vec!["repo_path", "branch"],
            vec![
                ("repo_path", prop_type("string")),
                ("branch", prop_type("string")),
            ],
        ),
        // === GitButler (3) ===
        make_tool(
            "gb_list_branches",
            "List GitButler virtual branches",
            vec!["repo_path"],
            vec![("repo_path", prop_type("string"))],
        ),
        make_tool(
            "gb_push_stack",
            "Push a GitButler stack/branch",
            vec!["repo_path", "branch_name"],
            vec![
                ("repo_path", prop_type("string")),
                ("branch_name", prop_type("string")),
            ],
        ),
        make_tool(
            "gb_workspace_status",
            "Get full workspace status (git changes + GB branches)",
            vec!["repo_path"],
            vec![("repo_path", prop_type("string"))],
        ),
        // === Filesystem (3) ===
        make_tool(
            "fs_read_file",
            "Read file contents (must be within allowed roots)",
            vec!["path"],
            vec![("path", prop_type("string"))],
        ),
        make_tool(
            "fs_write_file",
            "Write content to a file (must be within allowed roots)",
            vec!["path", "content"],
            vec![
                ("path", prop_type("string")),
                ("content", prop_type("string")),
            ],
        ),
        make_tool(
            "fs_list_dir",
            "List directory contents (must be within allowed roots)",
            vec!["path"],
            vec![("path", prop_type("string"))],
        ),
        // === Knowledge Graph (4) ===
        make_tool(
            "kg_create_entity",
            "Create an entity in the knowledge graph",
            vec!["entity_type", "name"],
            vec![
                ("entity_type", prop_type("string")),
                ("name", prop_type("string")),
                ("properties", json!({"type": "object"}).as_object().unwrap().clone()),
            ],
        ),
        make_tool(
            "kg_create_edge",
            "Create a relationship (edge) between two entities",
            vec!["source", "target", "edge_type"],
            vec![
                ("source", prop_type("string")),
                ("target", prop_type("string")),
                ("edge_type", prop_type("string")),
                ("weight", json!({"type": "number"}).as_object().unwrap().clone()),
            ],
        ),
        make_tool(
            "kg_query",
            "Query the knowledge graph with optional filters",
            vec!["query"],
            vec![
                ("query", prop_type("string")),
                ("limit", json!({"type": "integer"}).as_object().unwrap().clone()),
            ],
        ),
        make_tool(
            "kg_traverse",
            "Traverse graph relationships starting from an entity",
            vec!["source"],
            vec![
                ("source", prop_type("string")),
                ("max_depth", json!({"type": "integer"}).as_object().unwrap().clone()),
            ],
        ),
        // === A2A Protocol (7) ===
        make_tool(
            "a2a_register",
            "Register a new A2A agent with capabilities",
            vec!["id", "name"],
            vec![
                ("id", prop_type("string")),
                ("name", prop_type("string")),
                ("description", prop_type("string")),
            ],
        ),
        make_tool(
            "a2a_list_agents",
            "List all registered A2A agents",
            vec![],
            vec![],
        ),
        make_tool(
            "a2a_send",
            "Send a direct message from one agent to another",
            vec!["from", "to"],
            vec![
                ("from", prop_type("string")),
                ("to", prop_type("string")),
                ("payload", json!({"type": "object"}).as_object().unwrap().clone()),
            ],
        ),
        make_tool(
            "a2a_broadcast",
            "Broadcast a message to all registered agents",
            vec!["from"],
            vec![
                ("from", prop_type("string")),
                ("payload", json!({"type": "object"}).as_object().unwrap().clone()),
            ],
        ),
        make_tool(
            "a2a_assign_task",
            "Assign a task to an A2A agent",
            vec!["title", "created_by", "assign_to"],
            vec![
                ("title", prop_type("string")),
                ("created_by", prop_type("string")),
                ("assign_to", prop_type("string")),
            ],
        ),
        make_tool(
            "a2a_task_status",
            "Get the current status of an A2A task",
            vec!["task_id"],
            vec![("task_id", prop_type("string"))],
        ),
        make_tool(
            "a2a_update_task",
            "Update the state of an A2A task",
            vec!["task_id", "state"],
            vec![
                ("task_id", prop_type("string")),
                ("state", prop_type("string")),
            ],
        ),
        // === BrowserOS (12) ===
        make_tool(
            "browser_get_url",
            "Get current URL of the browser tab controlled by BrowserOS agent",
            vec!["agent_id"],
            vec![("agent_id", prop_type("string"))],
        ),
        make_tool(
            "browser_get_title",
            "Get page title of the browser tab",
            vec!["agent_id"],
            vec![("agent_id", prop_type("string"))],
        ),
        make_tool(
            "browser_navigate",
            "Navigate the browser tab to a URL",
            vec!["agent_id", "url"],
            vec![
                ("agent_id", prop_type("string")),
                ("url", prop_type("string")),
            ],
        ),
        make_tool(
            "browser_get_dom",
            "Get full page HTML of the browser tab",
            vec!["agent_id"],
            vec![("agent_id", prop_type("string"))],
        ),
        make_tool(
            "browser_query_selector",
            "Find a DOM element by CSS selector",
            vec!["agent_id", "selector"],
            vec![
                ("agent_id", prop_type("string")),
                ("selector", prop_type("string")),
            ],
        ),
        make_tool(
            "browser_click",
            "Click a DOM element by CSS selector",
            vec!["agent_id", "selector"],
            vec![
                ("agent_id", prop_type("string")),
                ("selector", prop_type("string")),
            ],
        ),
        make_tool(
            "browser_type",
            "Type text into an input or textarea element",
            vec!["agent_id", "selector", "text"],
            vec![
                ("agent_id", prop_type("string")),
                ("selector", prop_type("string")),
                ("text", prop_type("string")),
            ],
        ),
        make_tool(
            "browser_scroll",
            "Scroll the browser tab to coordinates",
            vec!["agent_id", "y"],
            vec![
                ("agent_id", prop_type("string")),
                ("y", json!({"type": "number"}).as_object().unwrap().clone()),
            ],
        ),
        make_tool(
            "browser_eval",
            "Evaluate a JavaScript expression in the browser tab",
            vec!["agent_id", "js"],
            vec![
                ("agent_id", prop_type("string")),
                ("js", prop_type("string")),
            ],
        ),
        make_tool(
            "browser_screenshot",
            "Capture a screenshot of the browser tab as base64 PNG",
            vec!["agent_id"],
            vec![("agent_id", prop_type("string"))],
        ),
        make_tool(
            "browser_open_tab",
            "Open a new browser tab with given URL",
            vec!["agent_id", "url"],
            vec![
                ("agent_id", prop_type("string")),
                ("url", prop_type("string")),
            ],
        ),
        make_tool(
            "browser_close_tab",
            "Close the current browser tab",
            vec!["agent_id"],
            vec![("agent_id", prop_type("string"))],
        ),
    ]
}

#[derive(Clone)]
pub struct McpService;

impl McpService {
    pub fn new() -> Self {
        Self
    }

    pub fn list_tools(&self) -> ListToolsResult {
        ListToolsResult {
            tools: TOOL_DEFINITIONS.clone(),
            meta: None,
            next_cursor: None,
        }
    }

    pub async fn call_tool(&self, params: CallToolRequestParams) -> CallToolResult {
        info!("Calling tool: {}", params.name);

        let arguments_value: Value = params
            .arguments
            .map(|map| Value::Object(map.into_iter().collect()))
            .unwrap_or(Value::Null);

        match tools::dispatch(&params.name, arguments_value).await {
            Ok(value) => {
                let text: String = if value.is_object() || value.is_array() {
                    serde_json::to_string_pretty(&value).unwrap_or_default()
                } else if value.is_string() {
                    value.as_str().unwrap().to_string()
                } else {
                    serde_json::to_string(&value).unwrap_or_default()
                };

                CallToolResult {
                    content: vec![ContentBlock::TextContent(TextContent::new(
                        text,
                        None,
                        None,
                    ))],
                    is_error: Some(false),
                    meta: None,
                    structured_content: None,
                }
            }
            Err(e) => CallToolResult {
                content: vec![ContentBlock::TextContent(TextContent::new(
                    format!("Error: {}", e),
                    None,
                    None,
                ))],
                is_error: Some(true),
                meta: None,
                structured_content: None,
            },
        }
    }
}
