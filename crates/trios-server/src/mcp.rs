use rust_mcp_schema::{
    CallToolRequestParams, CallToolResult, ContentBlock, ListToolsResult, TextContent,
    Tool, ToolInputSchema,
};
use serde_json::{json, Value};
use std::sync::LazyLock;
use tracing::info;

use crate::tools;

static TOOL_DEFINITIONS: LazyLock<Vec<Tool>> = LazyLock::new(build_tool_definitions);

fn build_tool_definitions() -> Vec<Tool> {
    vec![
        Tool {
            name: "git_status".into(),
            description: Some("List all changed files in a repository".into()),
            input_schema: ToolInputSchema::new(
                vec!["repo_path".into()],
                Some(
                    vec![(
                        "repo_path".into(),
                        json!({"type": "string"}).as_object().unwrap().clone(),
                    )]
                    .into_iter()
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
        },
        Tool {
            name: "git_stage_files".into(),
            description: Some("Stage specific files for commit".into()),
            input_schema: ToolInputSchema::new(
                vec!["repo_path".into(), "paths".into()],
                Some(
                    vec![
                        (
                            "repo_path".into(),
                            json!({"type": "string"}).as_object().unwrap().clone(),
                        ),
                        (
                            "paths".into(),
                            json!({"type": "array", "items": {"type": "string"}})
                                .as_object()
                                .unwrap()
                                .clone(),
                        ),
                    ]
                    .into_iter()
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
        },
        Tool {
            name: "git_commit".into(),
            description: Some("Commit all staged files".into()),
            input_schema: ToolInputSchema::new(
                vec!["repo_path".into(), "message".into()],
                Some(
                    vec![
                        (
                            "repo_path".into(),
                            json!({"type": "string"}).as_object().unwrap().clone(),
                        ),
                        (
                            "message".into(),
                            json!({"type": "string"}).as_object().unwrap().clone(),
                        ),
                    ]
                    .into_iter()
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
        },
        Tool {
            name: "git_create_branch".into(),
            description: Some("Create a new branch".into()),
            input_schema: ToolInputSchema::new(
                vec!["repo_path".into(), "name".into()],
                Some(
                    vec![
                        (
                            "repo_path".into(),
                            json!({"type": "string"}).as_object().unwrap().clone(),
                        ),
                        (
                            "name".into(),
                            json!({"type": "string"}).as_object().unwrap().clone(),
                        ),
                    ]
                    .into_iter()
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
        },
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
                let text = if value.is_object() || value.is_array() {
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
