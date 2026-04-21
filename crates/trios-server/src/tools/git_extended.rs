//! Git extended tools module.
//!
//! Provides extended git operations: log, diff, stash, checkout.

use anyhow::{Context, Result};
use serde_json::Value;
use trios_core::git::GitOrchestrator;
use trios_git::Git2Orchestrator;

pub const fn tool_count() -> usize { 4 }

/// Dispatch git extended tools.
pub async fn dispatch(name: &str, input: &Value, repo: &std::path::Path, git: &Git2Orchestrator) -> Option<Result<Value>> {
    match name {
        "git_log" => Some(git_log(repo, git, input).await),
        "git_diff" => Some(git_diff(repo, git, input).await),
        "git_stash" => Some(git_stash(repo, git).await),
        "git_checkout" => Some(git_checkout(repo, git, input).await),
        _ => None,
    }
}

/// Get git log.
async fn git_log(repo: &std::path::Path, git: &Git2Orchestrator, input: &Value) -> Result<Value> {
    let limit = input
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;
    let entries = git.log(repo, limit).await?;
    Ok(serde_json::to_value(entries)?)
}

/// Get git diff.
async fn git_diff(repo: &std::path::Path, git: &Git2Orchestrator, input: &Value) -> Result<Value> {
    let file = input.get("file").and_then(|v| v.as_str());
    let result = git.diff(repo, file).await?;
    Ok(serde_json::to_value(result)?)
}

/// Stash changes.
async fn git_stash(repo: &std::path::Path, git: &Git2Orchestrator) -> Result<Value> {
    git.stash(repo).await?;
    Ok(serde_json::json!({"stashed": true}))
}

/// Checkout branch.
async fn git_checkout(repo: &std::path::Path, git: &Git2Orchestrator, input: &Value) -> Result<Value> {
    let branch_name = input
        .get("branch")
        .and_then(|v| v.as_str())
        .context("branch required")?;
    git.switch_branch(repo, branch_name).await?;
    Ok(serde_json::json!({"switched_to": branch_name}))
}
