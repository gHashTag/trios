use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use trios_git::Git2Orchestrator;
use trios_core::GitOrchestrator;

use crate::security::validate_repo_path;

/// Returns the list of allowed repository root directories.
/// Configured via `TRIOS_ALLOWED_ROOTS` env var (colon-separated paths).
/// If empty, all absolute paths are allowed.
fn allowed_roots() -> Vec<PathBuf> {
    std::env::var("TRIOS_ALLOWED_ROOTS")
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

pub async fn dispatch(name: &str, input: Value) -> Result<Value> {
    let raw_repo_path = input
        .get("repo_path")
        .and_then(|v| v.as_str())
        .context("repo_path is required")?;

    let repo_path = validate_repo_path(raw_repo_path, &allowed_roots())
        .map_err(|e| anyhow::anyhow!(e))?;
    let repo = repo_path.as_path();
    let git = Git2Orchestrator;

    match name {
        "git_status" => {
            let changes = git.status(repo).await?;
            Ok(serde_json::to_value(changes)?)
        }
        "git_stage_files" => {
            let paths: Vec<String> = input
                .get("paths")
                .and_then(|v| v.as_array())
                .context("paths array required")?
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            let path_refs: Vec<&Path> = paths.iter().map(|p| Path::new(p.as_str())).collect();
            git.stage(repo, &path_refs).await?;
            Ok(serde_json::json!({"staged": paths.len()}))
        }
        "git_unstage_files" => {
            let paths: Vec<String> = input
                .get("paths")
                .and_then(|v| v.as_array())
                .context("paths array required")?
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            let path_refs: Vec<&Path> = paths.iter().map(|p| Path::new(p.as_str())).collect();
            git.unstage(repo, &path_refs).await?;
            Ok(serde_json::json!({"unstaged": paths.len()}))
        }
        "git_commit" => {
            let message = input
                .get("message")
                .and_then(|v| v.as_str())
                .context("message required")?;

            // Validate message length
            if message.len() > 4096 {
                bail!("commit message too long (max 4096 characters)");
            }
            if message.is_empty() {
                bail!("commit message cannot be empty");
            }

            let result = git.commit(repo, message).await?;
            Ok(serde_json::to_value(result)?)
        }
        "git_create_branch" => {
            let branch_name = input
                .get("name")
                .and_then(|v| v.as_str())
                .context("name required")?;

            // Basic branch name validation
            if branch_name.is_empty() {
                bail!("branch name cannot be empty");
            }
            if branch_name.contains(' ') || branch_name.contains("..") || branch_name.contains('~') || branch_name.contains('^') || branch_name.contains(':') {
                bail!("branch name contains invalid characters");
            }

            git.create_branch(repo, branch_name).await?;
            Ok(serde_json::json!({"created": branch_name}))
        }
        "gb_list_branches" => {
            let branches = trios_gb::gb_list_branches_safe(repo).await;
            Ok(serde_json::to_value(branches)?)
        }
        "gb_push_stack" => {
            let branch_name = input
                .get("branch_name")
                .and_then(|v| v.as_str())
                .context("branch_name required")?;
            trios_gb::gb_push_stack(repo, branch_name).await?;
            Ok(serde_json::json!({"pushed": branch_name}))
        }
        other => bail!("unknown tool: {other}"),
    }
}
