use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::tools;

#[derive(Debug, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    pub input: Value,
}

#[derive(Debug, Serialize)]
pub struct ToolCallResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

pub async fn list_tools() -> Json<Vec<ToolDef>> {
    Json(vec![
        ToolDef {
            name: "git_status".into(),
            description: "List all changed files in a repository (staged + unstaged + untracked)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string", "description": "Absolute path to the git repository"}
                },
                "required": ["repo_path"]
            }),
        },
        ToolDef {
            name: "git_stage_files".into(),
            description: "Stage specific files for commit".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string"},
                    "paths": {"type": "array", "items": {"type": "string"}, "description": "Paths relative to repo root"}
                },
                "required": ["repo_path", "paths"]
            }),
        },
        ToolDef {
            name: "git_unstage_files".into(),
            description: "Unstage specific files".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string"},
                    "paths": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["repo_path", "paths"]
            }),
        },
        ToolDef {
            name: "git_commit".into(),
            description: "Commit all staged files with a message".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string"},
                    "message": {"type": "string", "description": "Conventional commit message"}
                },
                "required": ["repo_path", "message"]
            }),
        },
        ToolDef {
            name: "git_create_branch".into(),
            description: "Create a new local branch from HEAD".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string"},
                    "name": {"type": "string"}
                },
                "required": ["repo_path", "name"]
            }),
        },
        ToolDef {
            name: "gb_list_branches".into(),
            description: "List GitButler virtual branches (requires gitbutler-cli)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string"}
                },
                "required": ["repo_path"]
            }),
        },
        ToolDef {
            name: "gb_push_stack".into(),
            description: "Push a GitButler virtual branch/stack (requires gitbutler-cli)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string"},
                    "branch_name": {"type": "string"}
                },
                "required": ["repo_path", "branch_name"]
            }),
        },
    ])
}

pub async fn call_tool(Json(req): Json<ToolCallRequest>) -> Json<ToolCallResponse> {
    match tools::dispatch(&req.name, req.input).await {
        Ok(result) => Json(ToolCallResponse {
            success: true,
            result: Some(result),
            error: None,
        }),
        Err(e) => Json(ToolCallResponse {
            success: false,
            result: None,
            error: Some(e.to_string()),
        }),
    }
}
