mod commit;
mod stage;
mod status;
mod branch;
mod absorb_simple;
mod absorb_smart;

pub use commit::*;
pub use stage::*;
pub use status::*;
pub use branch::*;

use async_trait::async_trait;
use anyhow::Result;
use std::path::Path;
use trios_core::{GitOrchestrator, FileChange, CommitResult, BranchInfo};

#[derive(Debug, Clone, Default)]
pub struct Git2Orchestrator;

#[async_trait]
impl GitOrchestrator for Git2Orchestrator {
    async fn status(&self, repo_path: &Path) -> Result<Vec<FileChange>> {
        let path = repo_path.to_owned();
        tokio::task::spawn_blocking(move || status::get_status(&path)).await?
    }

    async fn stage(&self, repo_path: &Path, paths: &[&Path]) -> Result<()> {
        let path = repo_path.to_owned();
        let owned: Vec<std::path::PathBuf> = paths.iter().map(|p| p.to_path_buf()).collect();
        tokio::task::spawn_blocking(move || {
            let refs: Vec<&Path> = owned.iter().map(|p| p.as_path()).collect();
            stage::stage_files(&path, &refs)
        }).await?
    }

    async fn unstage(&self, repo_path: &Path, paths: &[&Path]) -> Result<()> {
        let path = repo_path.to_owned();
        let owned: Vec<std::path::PathBuf> = paths.iter().map(|p| p.to_path_buf()).collect();
        tokio::task::spawn_blocking(move || {
            let refs: Vec<&Path> = owned.iter().map(|p| p.as_path()).collect();
            stage::unstage_files(&path, &refs)
        }).await?
    }

    async fn commit(&self, repo_path: &Path, message: &str) -> Result<CommitResult> {
        let path = repo_path.to_owned();
        let msg = message.to_owned();
        tokio::task::spawn_blocking(move || commit::create_commit(&path, &msg)).await?
    }

    async fn create_branch(&self, repo_path: &Path, name: &str) -> Result<()> {
        let path = repo_path.to_owned();
        let n = name.to_owned();
        tokio::task::spawn_blocking(move || branch::create_branch(&path, &n)).await?
    }

    async fn switch_branch(&self, repo_path: &Path, name: &str) -> Result<()> {
        let path = repo_path.to_owned();
        let n = name.to_owned();
        tokio::task::spawn_blocking(move || branch::switch_branch(&path, &n)).await?
    }

    async fn list_branches(&self, repo_path: &Path) -> Result<Vec<BranchInfo>> {
        let path = repo_path.to_owned();
        tokio::task::spawn_blocking(move || branch::list_branches(&path)).await?
    }

    async fn push(&self, _repo_path: &Path, _remote: &str, _branch: &str) -> Result<()> {
        // TODO: implement via git2 with credential helper
        anyhow::bail!("push not yet implemented — use gitbutler-cli gb_push_stack")
    }
}
