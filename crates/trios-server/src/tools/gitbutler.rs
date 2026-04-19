//! GitButler tools module.
//!
//! Provides GitButler-specific operations: list branches, push stack, workspace status.

use anyhow::{Context, Result};
use serde_json::Value;
use trios_core::git::GitOrchestrator;
use trios_git::Git2Orchestrator;

/// Dispatch GitButler tools.
pub async fn dispatch(name: &str, input: &Value, repo: &std::path::Path, git: &Git2Orchestrator) -> Option<Result<Value>> {
    match name {
        "gb_list_branches" => Some(gb_list_branches(repo).await),
        "gb_push_stack" => Some(gb_push_stack(repo, input).await),
        "gb_workspace_status" => Some(gb_workspace_status(repo, git).await),
        _ => None,
    }
}

/// List GitButler branches.
async fn gb_list_branches(repo: &std::path::Path) -> Result<Value> {
    let branches = trios_gb::gb_list_branches_safe(repo).await;
    Ok(serde_json::to_value(branches)?)
}

/// Push a GitButler stack.
async fn gb_push_stack(repo: &std::path::Path, input: &Value) -> Result<Value> {
    let branch_name = input
        .get("branch_name")
        .and_then(|v| v.as_str())
        .context("branch_name required")?;
    trios_gb::gb_push_stack(repo, branch_name).await?;
    Ok(serde_json::json!({"pushed": branch_name}))
}

/// Get workspace status (combine git status + GB branches).
async fn gb_workspace_status(repo: &std::path::Path, git: &Git2Orchestrator) -> Result<Value> {
    let changes = git.status(repo).await.map_err(|e| anyhow::anyhow!(e))?;
    let branches = trios_gb::gb_list_branches_safe(repo).await;
    Ok(serde_json::json!({
        "changes": changes,
        "branches": branches,
    }))
}
