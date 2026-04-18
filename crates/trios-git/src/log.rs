use anyhow::Result;
use git2::Repository;
use std::path::Path;
use trios_core::types::LogEntry;

pub fn get_log(repo_path: &Path, limit: usize) -> Result<Vec<LogEntry>> {
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let mut entries = Vec::new();
    for oid in revwalk.take(limit) {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        entries.push(LogEntry {
            oid: oid.to_string(),
            message: commit.message().unwrap_or_default().lines().next().unwrap_or_default().to_string(),
            author: commit.author().name().unwrap_or_default().to_string(),
            timestamp: commit.time().seconds(),
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::stage_files;
    use crate::commit::create_commit;
    use std::fs;
    use tempfile::TempDir;

    fn init_with_commits(n: usize) -> TempDir {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        for i in 0..n {
            let file = dir.path().join(format!("file_{i}.rs"));
            fs::write(&file, format!("// commit {i}")).unwrap();
            stage_files(dir.path(), &[&file]).unwrap();
            create_commit(dir.path(), &format!("commit {i}")).unwrap();
        }
        dir
    }

    #[test]
    fn test_log_returns_commits() {
        let dir = init_with_commits(3);
        let entries = get_log(dir.path(), 10).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].message, "commit 2"); // newest first
    }

    #[test]
    fn test_log_respects_limit() {
        let dir = init_with_commits(5);
        let entries = get_log(dir.path(), 2).unwrap();
        assert_eq!(entries.len(), 2);
    }
}
