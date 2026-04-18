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
    // Filesystem tools don't need repo_path validation
    match name {
        "fs_read_file" => return fs_read_file(&input).await,
        "fs_write_file" => return fs_write_file(&input).await,
        "fs_list_dir" => return fs_list_dir(&input).await,
        _ => {}
    }

    let raw_repo_path = input
        .get("repo_path")
        .and_then(|v| v.as_str())
        .context("repo_path is required")?;

    let repo_path = validate_repo_path(raw_repo_path, &allowed_roots())
        .map_err(|e| anyhow::anyhow!(e))?;
    let repo = repo_path.as_path();
    let git = Git2Orchestrator;

    match name {
        // === Git Basic (7) ===
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
            if branch_name.is_empty() {
                bail!("branch name cannot be empty");
            }
            if branch_name.contains(' ') || branch_name.contains("..") || branch_name.contains('~') || branch_name.contains('^') || branch_name.contains(':') {
                bail!("branch name contains invalid characters");
            }
            git.create_branch(repo, branch_name).await?;
            Ok(serde_json::json!({"created": branch_name}))
        }

        // === Git Extended (4) ===
        "git_log" => {
            let limit = input
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(10) as usize;
            let entries = git.log(repo, limit).await?;
            Ok(serde_json::to_value(entries)?)
        }
        "git_diff" => {
            let file = input.get("file").and_then(|v| v.as_str());
            let result = git.diff(repo, file).await?;
            Ok(serde_json::to_value(result)?)
        }
        "git_stash" => {
            git.stash(repo).await?;
            Ok(serde_json::json!({"stashed": true}))
        }
        "git_checkout" => {
            let branch_name = input
                .get("branch")
                .and_then(|v| v.as_str())
                .context("branch required")?;
            git.switch_branch(repo, branch_name).await?;
            Ok(serde_json::json!({"switched_to": branch_name}))
        }

        // === GitButler (3) ===
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
        "gb_workspace_status" => {
            // Combine git status + gb branches into a single workspace view
            let changes = git.status(repo).await?;
            let branches = trios_gb::gb_list_branches_safe(repo).await;
            Ok(serde_json::json!({
                "changes": changes,
                "branches": branches,
            }))
        }

        other => bail!("unknown tool: {other}"),
    }
}

// === Filesystem Tools ===

async fn fs_read_file(input: &Value) -> Result<Value> {
    let path = input
        .get("path")
        .and_then(|v| v.as_str())
        .context("path is required")?;

    let path = PathBuf::from(path);
    if !path.is_absolute() {
        bail!("path must be absolute");
    }

    // Validate against allowed roots
    let roots = allowed_roots();
    if !roots.is_empty() {
        let canonical = path.canonicalize().map_err(|e| anyhow::anyhow!("invalid path: {}", e))?;
        let permitted = roots.iter().any(|root| {
            let canonical_root = root.canonicalize().unwrap_or_else(|_| root.clone());
            canonical.starts_with(&canonical_root)
        });
        if !permitted {
            bail!("path is outside allowed directories");
        }
    }

    let content = tokio::fs::read_to_string(&path).await?;
    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "content": content,
        "size_bytes": content.len(),
    }))
}

async fn fs_write_file(input: &Value) -> Result<Value> {
    let path = input
        .get("path")
        .and_then(|v| v.as_str())
        .context("path is required")?;
    let content = input
        .get("content")
        .and_then(|v| v.as_str())
        .context("content is required")?;

    if content.len() > 1024 * 1024 {
        bail!("content too large (max 1 MB)");
    }

    let path = PathBuf::from(path);
    if !path.is_absolute() {
        bail!("path must be absolute");
    }

    // Validate against allowed roots
    let roots = allowed_roots();
    if !roots.is_empty() {
        let parent = path.parent().context("path has no parent directory")?;
        let canonical = parent.canonicalize().map_err(|e| anyhow::anyhow!("invalid parent dir: {}", e))?;
        let permitted = roots.iter().any(|root| {
            let canonical_root = root.canonicalize().unwrap_or_else(|_| root.clone());
            canonical.starts_with(&canonical_root)
        });
        if !permitted {
            bail!("path is outside allowed directories");
        }
    }

    tokio::fs::write(&path, content).await?;
    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "written": true,
        "size_bytes": content.len(),
    }))
}

async fn fs_list_dir(input: &Value) -> Result<Value> {
    let path = input
        .get("path")
        .and_then(|v| v.as_str())
        .context("path is required")?;

    let path = PathBuf::from(path);
    if !path.is_absolute() {
        bail!("path must be absolute");
    }
    if !path.is_dir() {
        bail!("path is not a directory");
    }

    // Validate against allowed roots
    let roots = allowed_roots();
    if !roots.is_empty() {
        let canonical = path.canonicalize().map_err(|e| anyhow::anyhow!("invalid path: {}", e))?;
        let permitted = roots.iter().any(|root| {
            let canonical_root = root.canonicalize().unwrap_or_else(|_| root.clone());
            canonical.starts_with(&canonical_root)
        });
        if !permitted {
            bail!("path is outside allowed directories");
        }
    }

    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(&path).await?;
    while let Some(entry) = dir.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().await?.is_dir();
        entries.push(serde_json::json!({
            "name": name,
            "is_dir": is_dir,
        }));
    }

    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "entries": entries,
        "count": entries.len(),
    }))
}
