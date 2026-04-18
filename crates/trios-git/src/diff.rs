use anyhow::Result;
use git2::{DiffFormat, Repository};
use std::path::Path;
use trios_core::types::DiffResult;

pub fn get_diff(repo_path: &Path, file: Option<&str>) -> Result<DiffResult> {
    let repo = Repository::open(repo_path)?;

    let diff = if let Some(path) = file {
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path);
        repo.diff_index_to_workdir(None, Some(&mut opts))?
    } else {
        repo.diff_index_to_workdir(None, None)?
    };

    let mut patch = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = line.origin();
        match origin {
            '+' | '-' | ' ' => patch.push(origin),
            _ => {}
        }
        if let Ok(content) = std::str::from_utf8(line.content()) {
            patch.push_str(content);
        }
        true
    })?;

    let mut files = Vec::new();
    for delta in diff.deltas() {
        if let Some(p) = delta.new_file().path() {
            files.push(p.to_string_lossy().to_string());
        } else if let Some(p) = delta.old_file().path() {
            files.push(p.to_string_lossy().to_string());
        }
    }

    Ok(DiffResult { files, patch })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::stage_files;
    use crate::commit::create_commit;
    use std::fs;
    use tempfile::TempDir;

    fn init_with_commit() -> TempDir {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        let file = dir.path().join("lib.rs");
        fs::write(&file, "fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();
        stage_files(dir.path(), &[&file]).unwrap();
        create_commit(dir.path(), "initial").unwrap();
        dir
    }

    #[test]
    fn test_diff_shows_changes() {
        let dir = init_with_commit();
        let file = dir.path().join("lib.rs");
        fs::write(&file, "fn add(a: i64, b: i64) -> i64 { a + b }").unwrap();
        let result = get_diff(dir.path(), None).unwrap();
        assert!(!result.patch.is_empty());
        assert!(result.files.iter().any(|f| f.contains("lib.rs")));
    }

    #[test]
    fn test_diff_filter_by_file() {
        let dir = init_with_commit();
        let file = dir.path().join("lib.rs");
        fs::write(&file, "changed content").unwrap();
        let other = dir.path().join("other.rs");
        fs::write(&other, "other content").unwrap();
        let result = get_diff(dir.path(), Some("lib.rs")).unwrap();
        assert!(result.files.iter().any(|f| f.contains("lib.rs")));
    }
}
