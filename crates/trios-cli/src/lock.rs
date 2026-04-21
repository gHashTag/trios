//! File locking for issue #143 race condition prevention
//!
//! Uses `.trinity/tri.lock` to prevent concurrent updates
//! to the #143 table from multiple agents.

use anyhow::{Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use libc::kill;

const LOCK_FILE: &str = ".trinity/tri.lock";
const LOCK_TIMEOUT_MS: u128 = 30000;

pub struct LockGuard {
    _file: File,
    path: PathBuf,
}

impl LockGuard {
    pub fn acquire() -> Result<Self> {
        Self::acquire_at(LOCK_FILE)
    }

    pub fn acquire_at(lock_path: &str) -> Result<Self> {
        let start = std::time::Instant::now();
        let path = PathBuf::from(lock_path);

        loop {
            match Self::try_acquire(&path) {
                Ok(guard) => return Ok(guard),
                Err(e) => {
                    if start.elapsed().as_millis() > LOCK_TIMEOUT_MS {
                        anyhow::bail!("Failed to acquire lock after {}ms: {}", LOCK_TIMEOUT_MS, e);
                    }

                    if let Ok(stale) = Self::is_lock_stale(&path) {
                        if stale {
                            fs::remove_file(&path).ok();
                            continue;
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }

    fn try_acquire(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .context("Failed to create lock file")?;

        let pid = std::process::id();
        writeln!(file, "{}", pid)?;

        Ok(LockGuard {
            _file: file,
            path: path.to_path_buf(),
        })
    }

    fn is_lock_stale(path: &Path) -> Result<bool> {
        let content = fs::read_to_string(path)?;
        let pid: u32 = content.trim().parse().context("Failed to parse lock PID")?;

        #[cfg(unix)]
        {
            if unsafe { kill(pid as i32, 0) } == 0 {
                return Ok(false);
            }
            Ok(true)
        }

        #[cfg(not(unix))]
        {
            let metadata = fs::metadata(path)?;
            let age = metadata
                .modified()?
                .elapsed()
                .context("Failed to get lock age")?;
            Ok(age.as_secs() > 3600)
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        fs::remove_file(&self.path).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_guard() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("test.lock");
        let lock_str = lock_path.to_str().unwrap();

        let guard = LockGuard::acquire_at(lock_str).unwrap();
        assert!(lock_path.exists());

        drop(guard);
        assert!(!lock_path.exists());
    }
}
