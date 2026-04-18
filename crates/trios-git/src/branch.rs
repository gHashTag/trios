use anyhow::Result;
use git2::{BranchType, Repository};
use std::path::Path;
use trios_core::types::BranchInfo;

pub fn create_branch(repo_path: &Path, name: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?.peel_to_commit()?;
    repo.branch(name, &head, false)?;
    Ok(())
}

pub fn switch_branch(repo_path: &Path, name: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let branch = repo.find_branch(name, BranchType::Local)?;
    let obj = branch.get().peel_to_commit()?.into_object();
    repo.checkout_tree(&obj, None)?;
    repo.set_head(&format!("refs/heads/{name}"))?;
    Ok(())
}

pub fn list_branches(repo_path: &Path) -> Result<Vec<BranchInfo>> {
    let repo = Repository::open(repo_path)?;
    let head_name = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_owned()));

    let mut branches = Vec::new();
    for branch in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = branch?;
        let name = branch
            .name()?
            .unwrap_or("<unnamed>")
            .to_owned();
        let is_current = head_name.as_deref() == Some(&name);
        let upstream = branch
            .upstream()
            .ok()
            .and_then(|u| u.name().ok().flatten().map(|s| s.to_owned()));
        branches.push(BranchInfo {
            name,
            is_current,
            upstream,
            commit_count: None,
        });
    }
    Ok(branches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use crate::stage::stage_files;
    use crate::commit::create_commit;

    fn init_with_commit() -> TempDir {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        let file = dir.path().join("init.rs");
        fs::write(&file, "// init").unwrap();
        stage_files(dir.path(), &[&file]).unwrap();
        create_commit(dir.path(), "chore: init").unwrap();
        dir
    }

    #[test]
    fn test_create_branch() {
        let dir = init_with_commit();
        create_branch(dir.path(), "feature/test").unwrap();
        let branches = list_branches(dir.path()).unwrap();
        assert!(branches.iter().any(|b| b.name == "feature/test"));
    }

    #[test]
    fn test_list_branches() {
        let dir = init_with_commit();
        let branches = list_branches(dir.path()).unwrap();
        assert!(!branches.is_empty());
        assert!(branches.iter().any(|b| b.is_current));
    }
}
