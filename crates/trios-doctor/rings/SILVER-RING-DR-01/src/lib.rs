//! SILVER-RING-DR-01 — Check runner
//! Executes cargo check / clippy / test and returns WorkspaceCheck results.

use std::path::{Path, PathBuf};
use std::process::Command;
use trios_doctor_dr00::{CheckStatus, WorkspaceCheck, WorkspaceDiagnosis};

pub struct Doctor {
    workspace_root: PathBuf,
}

impl Doctor {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
        }
    }

    pub fn count_crates(&self) -> usize {
        self.discover_crates().len()
    }

    pub fn run_all(&self) -> WorkspaceDiagnosis {
        let checks = vec![
            self.workspace_check(),
            self.workspace_test(),
            self.workspace_clippy(),
        ];
        WorkspaceDiagnosis {
            workspace: self.workspace_root.display().to_string(),
            crate_count: self.count_crates(),
            checks,
        }
    }

    fn discover_crates(&self) -> Vec<PathBuf> {
        let crates_dir = self.workspace_root.join("crates");
        let mut result = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&crates_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.join("Cargo.toml").exists() {
                    result.push(path);
                }
            }
        }
        result.sort();
        result
    }

    fn workspace_check(&self) -> WorkspaceCheck {
        let output = Command::new("cargo")
            .args(["check", "--workspace"])
            .current_dir(&self.workspace_root)
            .output();
        match output {
            Ok(out) if out.status.success() => WorkspaceCheck {
                name: "cargo check --workspace".into(),
                status: CheckStatus::Green,
                message: "All crates compile".into(),
                failed_crates: vec![],
            },
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let failed = self.extract_failed_crates(&stderr);
                WorkspaceCheck {
                    name: "cargo check --workspace".into(),
                    status: CheckStatus::Red,
                    message: self.first_lines(&stderr, 10),
                    failed_crates: failed,
                }
            }
            Err(e) => WorkspaceCheck {
                name: "cargo check --workspace".into(),
                status: CheckStatus::Red,
                message: format!("cargo failed: {}", e),
                failed_crates: vec![],
            },
        }
    }

    fn workspace_test(&self) -> WorkspaceCheck {
        let output = Command::new("cargo")
            .args(["test", "--workspace"])
            .current_dir(&self.workspace_root)
            .output();
        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                WorkspaceCheck {
                    name: "cargo test --workspace".into(),
                    status: CheckStatus::Green,
                    message: self.extract_test_summary(&stdout),
                    failed_crates: vec![],
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let failed = self.extract_failed_crates(&stderr);
                WorkspaceCheck {
                    name: "cargo test --workspace".into(),
                    status: CheckStatus::Red,
                    message: self.first_lines(&stderr, 10),
                    failed_crates: failed,
                }
            }
            Err(e) => WorkspaceCheck {
                name: "cargo test --workspace".into(),
                status: CheckStatus::Red,
                message: format!("cargo test failed: {}", e),
                failed_crates: vec![],
            },
        }
    }

    fn workspace_clippy(&self) -> WorkspaceCheck {
        let output = Command::new("cargo")
            .args(["clippy", "--workspace", "--", "-D", "warnings"])
            .current_dir(&self.workspace_root)
            .output();
        match output {
            Ok(out) if out.status.success() => WorkspaceCheck {
                name: "cargo clippy --workspace".into(),
                status: CheckStatus::Green,
                message: "0 warnings".into(),
                failed_crates: vec![],
            },
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let failed = self.extract_failed_crates(&stderr);
                WorkspaceCheck {
                    name: "cargo clippy --workspace".into(),
                    status: CheckStatus::Yellow,
                    message: self.first_lines(&stderr, 15),
                    failed_crates: failed,
                }
            }
            Err(e) => WorkspaceCheck {
                name: "cargo clippy --workspace".into(),
                status: CheckStatus::Red,
                message: format!("clippy failed: {}", e),
                failed_crates: vec![],
            },
        }
    }

    fn extract_failed_crates(&self, output: &str) -> Vec<String> {
        let mut crates = Vec::new();
        for line in output.lines() {
            if line.contains("error") && line.contains("crates/") {
                for part in line.split_whitespace() {
                    if part.starts_with("crates/") {
                        if let Some(name) = part.split('/').nth(1) {
                            if !crates.contains(&name.to_string()) {
                                crates.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        crates
    }

    fn extract_test_summary(&self, output: &str) -> String {
        let count = output.matches("test result:").count();
        format!("{} test suites passed", count)
    }

    fn first_lines(&self, text: &str, n: usize) -> String {
        text.lines().take(n).collect::<Vec<_>>().join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_new_stores_root() {
        let d = Doctor::new("/tmp");
        assert!(d.workspace_root.ends_with("tmp"));
    }

    #[test]
    fn count_crates_with_empty_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let crates_dir = tmp.path().join("crates");
        std::fs::create_dir_all(crates_dir).unwrap();
        let d = Doctor::new(tmp.path());
        assert_eq!(d.count_crates(), 0);
    }
}
