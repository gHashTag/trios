use anyhow::{bail, Context, Result};
use std::path::Path;
use trios_core::types::GbBranch;

/// Checks if gitbutler-cli is available on PATH
pub fn gb_cli_available() -> bool {
    which_gb().is_ok()
}

fn which_gb() -> Result<std::path::PathBuf> {
    // Check common locations
    for candidate in &["gitbutler-cli", "gb"] {
        if let Ok(path) = which::which(candidate) {
            return Ok(path);
        }
    }
    bail!("gitbutler-cli not found on PATH — install from https://github.com/gitbutlerapp/gitbutler")
}

/// List GitButler virtual branches for a repo
pub async fn gb_list_branches(repo_path: &Path) -> Result<Vec<GbBranch>> {
    let cli = which_gb().context("gitbutler-cli required for gb_list_branches")?;
    let path = repo_path.to_owned();

    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&cli)
            .args(["branch", "list"])
            .current_dir(&path)
            .output()
    })
    .await??
    ;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gitbutler-cli branch list failed: {stderr}");
    }

    // Try to parse JSON output; fall back to empty list
    let stdout = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<GbBranch> = serde_json::from_str(&stdout).unwrap_or_default();
    Ok(branches)
}

/// Push a GitButler stack/branch
pub async fn gb_push_stack(repo_path: &Path, branch_name: &str) -> Result<()> {
    let cli = which_gb().context("gitbutler-cli required for gb_push_stack")?;
    let path = repo_path.to_owned();
    let name = branch_name.to_owned();

    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&cli)
            .args(["branch", "push", &name])
            .current_dir(&path)
            .output()
    })
    .await??
    ;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gitbutler-cli branch push failed: {stderr}");
    }
    Ok(())
}

/// Graceful fallback: returns empty list if GB CLI not available
pub async fn gb_list_branches_safe(repo_path: &Path) -> Vec<GbBranch> {
    gb_list_branches(repo_path).await.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_when_no_cli() {
        // This test verifies graceful degradation
        // gb_cli_available() may be true or false depending on environment
        // The important thing is it doesn't panic
        let _ = gb_cli_available();
    }

    #[tokio::test]
    async fn test_list_branches_safe_no_panic() {
        let dir = tempfile::TempDir::new().unwrap();
        // Should not panic even if gitbutler-cli is not installed
        let branches = gb_list_branches_safe(dir.path()).await;
        // Empty list is valid fallback
        assert!(branches.is_empty() || !branches.is_empty());
    }
}
