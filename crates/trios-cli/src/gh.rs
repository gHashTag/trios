//! GitHub CLI wrapper for issue/PR operations
//!
//! Wraps `gh` commands via std::process for issue #143 table sync,
//! issue creation, and agent roster management.

use anyhow::{Context, Result};
use std::process::{Command, Stdio};

pub struct GhClient;

impl GhClient {
    pub fn new() -> Self {
        Self
    }

    /// Get issue body as string
    pub fn issue_body(&self, num: u32) -> Result<String> {
        let output = Command::new("gh")
            .args(["issue", "view", &num.to_string(), "--json", "body", "-q", ".body"])
            .stdout(Stdio::piped())
            .output()
            .context("Failed to run gh issue view")?;

        if !output.status.success() {
            anyhow::bail!("gh issue view failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(String::from_utf8(output.stdout)?)
    }

    /// Update issue body
    pub fn issue_update(&self, num: u32, body: &str) -> Result<()> {
        let output = Command::new("gh")
            .args(["issue", "edit", &num.to_string(), "--body", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to spawn gh issue edit")?
            .stdin
            .as_mut()
            .context("Failed to open stdin")?
            .write_all(body.as_bytes())
            .context("Failed to write to gh stdin")?;

        Ok(())
    }

    /// Create new issue
    pub fn issue_create(&self, title: &str, body: &str, labels: &[&str]) -> Result<u32> {
        let mut cmd = Command::new("gh");
        cmd.args(["issue", "create", "--title", title, "--body", "-"]);

        for label in labels {
            cmd.args(["--label", label]);
        }

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to spawn gh issue create")?;

        child.stdin
            .as_mut()
            .context("Failed to open stdin")?
            .write_all(body.as_bytes())
            .context("Failed to write to gh stdin")?;

        let output = child.wait_with_output().context("Failed to wait for gh issue create")?;

        if !output.status.success() {
            anyhow::bail!("gh issue create failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Parse issue number from output (format: "https://github.com/owner/repo/issues/123")
        let stdout = String::from_utf8_lossy(&output.stdout);
        let num = stdout
            .rsplit('/')
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .context("Failed to parse issue number from gh output")?;

        Ok(num)
    }

    /// Close issue
    pub fn issue_close(&self, num: u32, comment: Option<&str>) -> Result<()> {
        let mut cmd = Command::new("gh");
        cmd.args(["issue", "close", &num.to_string()]);

        if let Some(c) = comment {
            cmd.args(["--comment", c]);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .output()
            .context("Failed to run gh issue close")?;

        if !output.status.success() {
            anyhow::bail!("gh issue close failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    /// List agent roster (find agent-related issues)
    pub fn list_agent_issues(&self, nato: &str) -> Result<Vec<AgentIssue>> {
        let output = Command::new("gh")
            .args([
                "issue", "list",
                "--search", format!("in:title agent:{}", nato).as_str(),
                "--json", "number,title,state,labels",
            ])
            .stdout(Stdio::piped())
            .output()
            .context("Failed to run gh issue list")?;

        if !output.status.success() {
            anyhow::bail!("gh issue list failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let json = String::from_utf8(output.stdout)?;
        serde_json::from_str(&json).context("Failed to parse gh issue list JSON")
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct AgentIssue {
    pub number: u32,
    pub title: String,
    pub state: String,
    #[serde(default)]
    pub labels: Vec<String>,
}
