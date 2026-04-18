use anyhow::Result;
use git2::{Repository, Status};
use std::path::Path;
use trios_core::types::{FileChange, FileStatus};

pub fn get_status(repo_path: &Path) -> Result<Vec<FileChange>> {
    let repo = Repository::open(repo_path)?;
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut changes = Vec::new();

    for entry in statuses.iter() {
        let path = match entry.path() {
            Some(p) => std::path::PathBuf::from(p),
            None => continue,
        };

        let s = entry.status();
        let staged = s.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED,
        );

        let file_status = if s.contains(Status::CONFLICTED) {
            FileStatus::Conflicted
        } else if s.contains(Status::WT_NEW) || s.contains(Status::INDEX_NEW) {
            FileStatus::Added
        } else if s.contains(Status::WT_DELETED) || s.contains(Status::INDEX_DELETED) {
            FileStatus::Deleted
        } else if s.contains(Status::WT_RENAMED) || s.contains(Status::INDEX_RENAMED) {
            FileStatus::Renamed
        } else if s.contains(Status::INDEX_MODIFIED) || s.contains(Status::WT_MODIFIED) {
            FileStatus::Modified
        } else {
            FileStatus::Untracked
        };

        changes.push(FileChange {
            path,
            status: file_status,
            staged,
        });
    }

    Ok(changes)
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
    fn test_status_untracked() {
        let (dir, _repo) = init_repo();
        fs::write(dir.path().join("hello.rs"), "fn main() {}").unwrap();
        let changes = get_status(dir.path()).unwrap();
        assert!(!changes.is_empty());
        assert!(changes.iter().any(|c| c.path.to_str().unwrap().contains("hello.rs")));
    }

    #[test]
    fn test_status_empty_repo() {
        let (dir, _repo) = init_repo();
        let changes = get_status(dir.path()).unwrap();
        assert!(changes.is_empty());
    }
}
