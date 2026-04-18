use anyhow::Result;
use git2::Repository;
use std::path::Path;

pub fn stash_changes(repo_path: &Path) -> Result<()> {
    let mut repo = Repository::open(repo_path)?;

    // Check if there's anything to stash
    let has_changes = {
        let head = repo.head()?.target().ok_or_else(|| anyhow::anyhow!("no HEAD commit"))?;
        let commit = repo.find_commit(head)?;
        let tree = commit.tree()?;
        let index = repo.index()?;
        let diff = repo.diff_tree_to_index(Some(&tree), Some(&index), None)?;
        let wt_diff = repo.diff_index_to_workdir(None, None)?;
        diff.deltas().len() > 0 || wt_diff.deltas().len() > 0
    };

    if !has_changes {
        anyhow::bail!("nothing to stash");
    }

    let sig = repo.signature()?;
    repo.stash_save(&sig, "stash from trios-server", None)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::stage_files;
    use crate::commit::create_commit;
    use std::fs;
    use tempfile::TempDir;

    fn init_with_changes() -> TempDir {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        let file = dir.path().join("lib.rs");
        fs::write(&file, "fn main() {}").unwrap();
        stage_files(dir.path(), &[&file]).unwrap();
        create_commit(dir.path(), "initial").unwrap();
        // Now make a change
        fs::write(&file, "fn main() { println!(\"hello\"); }").unwrap();
        dir
    }

    #[test]
    fn test_stash() {
        let dir = init_with_changes();
        stash_changes(dir.path()).unwrap();
        // After stash, working dir should be clean
        let repo = git2::Repository::open(dir.path()).unwrap();
        let statuses = repo.statuses(None).unwrap();
        assert!(statuses.iter().all(|e| e.status().is_empty() || e.status() == git2::Status::IGNORED));
    }
}
