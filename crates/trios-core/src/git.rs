use crate::types::*;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait GitOrchestrator: Send + Sync {
    /// List all changed files (staged + unstaged + untracked)
    async fn status(&self, repo_path: &Path) -> Result<Vec<FileChange>>;

    /// Stage files by path
    async fn stage(&self, repo_path: &Path, paths: &[&Path]) -> Result<()>;

    /// Unstage files by path
    async fn unstage(&self, repo_path: &Path, paths: &[&Path]) -> Result<()>;

    /// Commit staged files
    async fn commit(&self, repo_path: &Path, message: &str) -> Result<CommitResult>;

    /// Create a new branch from HEAD
    async fn create_branch(&self, repo_path: &Path, name: &str) -> Result<()>;

    /// Switch to an existing branch
    async fn switch_branch(&self, repo_path: &Path, name: &str) -> Result<()>;

    /// List local branches
    async fn list_branches(&self, repo_path: &Path) -> Result<Vec<BranchInfo>>;

    /// Push branch to remote
    async fn push(&self, repo_path: &Path, remote: &str, branch: &str) -> Result<()>;

    /// Get commit log
    async fn log(&self, repo_path: &Path, limit: usize) -> Result<Vec<LogEntry>>;

    /// Get diff of unstaged or staged changes
    async fn diff(&self, repo_path: &Path, file: Option<&str>) -> Result<DiffResult>;

    /// Stash current changes
    async fn stash(&self, repo_path: &Path) -> Result<()>;
}
