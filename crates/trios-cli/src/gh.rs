//! gh.rs — GitHub CLI integration for tri commands
//! All GitHub operations go through `gh` binary (no browser, no API tokens in code)

use anyhow::{Context, Result};
use std::process::{Command, Output};

/// Run a `gh` subcommand and return stdout as String.
/// Respects L8: all mutations must be followed by push.
pub fn gh(args: &[&str]) -> Result<String> {
    let out: Output = Command::new("gh")
        .args(args)
        .output()
        .with_context(|| format!("gh {}", args.join(" ")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("gh {} failed: {}", args.join(" "), stderr);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Post a comment to an issue (gh issue comment <N> --body <body>)
pub fn issue_comment(issue: u32, repo: &str, body: &str) -> Result<String> {
    gh(&["issue", "comment", &issue.to_string(), "--repo", repo, "--body", body])
}

/// View issue body (gh issue view <N> --repo <repo> --json body -q .body)
pub fn issue_view(issue: u32, repo: &str) -> Result<String> {
    gh(&["issue", "view", &issue.to_string(), "--repo", repo, "--json", "body,title,state"])
}

/// Create a PR (gh pr create)
pub fn pr_create(title: &str, body: &str, head: &str, base: &str, repo: &str) -> Result<String> {
    gh(&["pr", "create", "--repo", repo, "--title", title, "--body", body, "--head", head, "--base", base])
}

/// List open issues with JSON output
pub fn issues_list(repo: &str, label: &str) -> Result<String> {
    gh(&["issue", "list", "--repo", repo, "--label", label, "--json", "number,title,assignees,labels,state"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gh_binary_exists() {
        // Just verify gh is on PATH — no network call
        let out = Command::new("gh").arg("--version").output();
        assert!(out.is_ok(), "gh binary must be available on PATH");
    }
}
