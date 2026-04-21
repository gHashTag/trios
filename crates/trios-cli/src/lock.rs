use anyhow::{Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use libc::kill;

const LOCK_TIMEOUT_MS: u128 = 30000;

pub fn lock_file_path() -> PathBuf {
    let mut path = std::env::current_dir().unwrap_or_else(|_| ".".into());
    loop {
        let candidate = path.join(".trinity");
        if candidate.exists() {
            return candidate.join("tri.lock");
        }
        if !path.pop() {
            break;
        }
    }
    PathBuf::from(".trinity/tri.lock")
}

pub struct LockGuard {
    _file: File,
    lock_path: PathBuf,
}

impl LockGuard {
    pub fn acquire() -> Result<Self> {
        let start = std::time::Instant::now();
        let lock_path = lock_file_path();

        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        loop {
            match Self::try_acquire(&lock_path) {
                Ok(guard) => return Ok(guard),
                Err(e) => {
                    if start.elapsed().as_millis() > LOCK_TIMEOUT_MS {
                        anyhow::bail!(
                            "Failed to acquire lock after {}ms: {}",
                            LOCK_TIMEOUT_MS,
                            e
                        );
                    }

                    if let Ok(stale) = Self::is_lock_stale(&lock_path) {
                        if stale {
                            fs::remove_file(&lock_path).ok();
                            continue;
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }

    fn try_acquire(lock_path: &Path) -> Result<Self> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock_path)
            .context("Failed to create lock file")?;

        let pid = std::process::id();
        writeln!(file, "{}", pid)?;

        Ok(LockGuard {
            _file: file,
            lock_path: lock_path.to_path_buf(),
        })
    }

    fn is_lock_stale(lock_path: &Path) -> Result<bool> {
        let content = fs::read_to_string(lock_path)?;
        let pid: u32 = content
            .trim()
            .parse()
            .context("Failed to parse lock PID")?;

        #[cfg(unix)]
        {
            if unsafe { kill(pid as i32, 0) } == 0 {
                return Ok(false);
            }
            Ok(true)
        }

        #[cfg(not(unix))]
        {
            let metadata = fs::metadata(lock_path)?;
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
        fs::remove_file(&self.lock_path).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_guard() {
        let guard1 = LockGuard::acquire().unwrap();
        let lp = lock_file_path();
        assert!(lp.exists());

        drop(guard1);
        assert!(!lp.exists());
    }
}
