use anyhow::Result;
use git2::Repository;
use std::path::Path;

pub fn stage_files(repo_path: &Path, paths: &[&Path]) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut index = repo.index()?;
    for path in paths {
        let rel = path.strip_prefix(repo_path).unwrap_or(path);
        index.add_path(rel)?;
    }
    index.write()?;
    Ok(())
}

pub fn unstage_files(repo_path: &Path, paths: &[&Path]) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    // Reset HEAD for each path to unstage
    let head = match repo.head() {
        Ok(h) => {
            let commit = h.peel_to_commit()?;
            Some(commit.into_object())
        }
        Err(_) => None, // no commits yet
    };

    let mut index = repo.index()?;
    for path in paths {
        let rel = path.strip_prefix(repo_path).unwrap_or(path);
        if let Some(ref obj) = head {
            repo.reset_default(Some(obj), std::iter::once(rel))?;
        } else {
            // No HEAD — just remove from index
            index.remove_path(rel)?;
        }
    }
    index.write()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn init_repo() -> (TempDir, git2::Repository) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (dir, repo)
    }

    #[test]
    fn test_stage_file() {
        let (dir, _repo) = init_repo();
        let file = dir.path().join("test.rs");
        fs::write(&file, "fn main() {}").unwrap();
        stage_files(dir.path(), &[&file]).unwrap();
        let repo = git2::Repository::open(dir.path()).unwrap();
        let statuses = repo.statuses(None).unwrap();
        let staged = statuses.iter().any(|e| {
            e.status().contains(git2::Status::INDEX_NEW)
        });
        assert!(staged);
    }
}
