use anyhow::{Context, Result};
use git2::Repository;
use std::path::Path;
use trios_core::types::CommitResult;

pub fn create_commit(repo_path: &Path, message: &str) -> Result<CommitResult> {
    let repo = Repository::open(repo_path)?;
    let mut index = repo.index()?;

    // Count staged entries before writing tree (this is the actual number of files in the commit)
    let files_committed = index.iter().count();

    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;

    let sig = repo
        .signature()
        .context("no git signature configured — set user.name and user.email")?;

    let parent_commits: Vec<git2::Commit> = match repo.head() {
        Ok(head) => vec![head.peel_to_commit()?],
        Err(_) => vec![],
    };
    let parents: Vec<&git2::Commit> = parent_commits.iter().collect();

    let commit_oid = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &parents,
    )?;

    Ok(CommitResult {
        oid: commit_oid.to_string(),
        message: message.to_owned(),
        files_committed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use crate::stage::stage_files;

    fn init_repo() -> (TempDir, git2::Repository) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (dir, repo)
    }

    #[test]
    fn test_commit() {
        let (dir, _repo) = init_repo();
        let file = dir.path().join("lib.rs");
        fs::write(&file, "pub fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();
        stage_files(dir.path(), &[&file]).unwrap();
        let result = create_commit(dir.path(), "feat: initial").unwrap();
        assert!(!result.oid.is_empty());
        assert_eq!(result.message, "feat: initial");
    }
}
