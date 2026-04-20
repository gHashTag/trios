pub mod check;
pub mod heal;
pub mod report;

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceDiagnosis {
    pub workspace: String,
    pub crate_count: usize,
    pub checks: Vec<WorkspaceCheck>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub failed_crates: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CheckStatus {
    Green,
    Yellow,
    Red,
}

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
                let summary = self.extract_test_summary(&stdout);
                WorkspaceCheck {
                    name: "cargo test --workspace".into(),
                    status: CheckStatus::Green,
                    message: summary,
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
        let mut total = 0u32;
        let mut passed = 0u32;
        let mut failed = 0u32;
        for line in output.lines() {
            if line.starts_with("test result:") {
                for part in line.split(';') {
                    let part = part.trim();
                    if let Some(n) = part.strip_prefix("passed ") {
                        passed += n.parse::<u32>().unwrap_or(0);
                    } else if let Some(n) = part.strip_prefix("failed ") {
                        failed += n.parse::<u32>().unwrap_or(0);
                    }
                }
                if let Some(rest) = line.strip_prefix("test result: ") {
                    if let Some(n) = rest.split('.').next() {
                        total += n.trim().parse::<u32>().unwrap_or(0);
                    }
                }
            }
        }
        if total > 0 {
            format!("{} tests: {} passed, {} failed", total, passed, failed)
        } else {
            let count = output.matches("test result:").count();
            format!("{} test suites passed", count)
        }
    }

    fn first_lines(&self, text: &str, n: usize) -> String {
        text.lines().take(n).collect::<Vec<_>>().join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn doctor_new_stores_root() {
        let d = Doctor::new("/tmp");
        assert!(d.workspace_root.ends_with("tmp"));
    }

    #[test]
    fn check_status_equality() {
        assert_eq!(CheckStatus::Green, CheckStatus::Green);
        assert_ne!(CheckStatus::Green, CheckStatus::Red);
        assert_ne!(CheckStatus::Yellow, CheckStatus::Red);
    }

    #[test]
    fn workspace_check_construction() {
        let check = WorkspaceCheck {
            name: "test".into(),
            status: CheckStatus::Green,
            message: "ok".into(),
            failed_crates: vec![],
        };
        assert_eq!(check.name, "test");
        assert_eq!(check.status, CheckStatus::Green);
        assert!(check.failed_crates.is_empty());
    }

    #[test]
    fn workspace_diagnosis_serde_roundtrip() {
        let diag = WorkspaceDiagnosis {
            workspace: "/fake".into(),
            crate_count: 5,
            checks: vec![WorkspaceCheck {
                name: "check".into(),
                status: CheckStatus::Green,
                message: "ok".into(),
                failed_crates: vec!["foo".into()],
            }],
        };
        let json = serde_json::to_string(&diag).unwrap();
        let back: WorkspaceDiagnosis = serde_json::from_str(&json).unwrap();
        assert_eq!(back.workspace, "/fake");
        assert_eq!(back.crate_count, 5);
        assert_eq!(back.checks.len(), 1);
        assert_eq!(back.checks[0].failed_crates, vec!["foo".to_string()]);
    }

    #[test]
    fn count_crates_with_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let crates_dir = tmp.path().join("crates");
        fs::create_dir_all(crates_dir).unwrap();
        let d = Doctor::new(tmp.path());
        assert_eq!(d.count_crates(), 0);
    }

    #[test]
    fn count_crates_finds_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let crate_dir = tmp.path().join("crates").join("fake-crate");
        fs::create_dir_all(&crate_dir).unwrap();
        fs::write(crate_dir.join("Cargo.toml"), "").unwrap();
        let d = Doctor::new(tmp.path());
        assert_eq!(d.count_crates(), 1);
    }

    #[test]
    fn extract_failed_crates_parses_error_line() {
        let d = Doctor::new(".");
        let output = "error: could not compile crates/foo/src/lib.rs";
        let crates = d.extract_failed_crates(output);
        assert!(crates.contains(&"foo".to_string()));
    }

    #[test]
    fn extract_test_summary_counts_suites() {
        let d = Doctor::new(".");
        let output = "test result: ok. 3 passed; 0 failed; 0 ignored\nrunning 2 tests\ntest result: ok. 2 passed; 0 failed; 0 ignored\n";
        let summary = d.extract_test_summary(output);
        assert_eq!(summary, "2 test suites passed");
    }

    #[test]
    fn first_lines_truncates() {
        let d = Doctor::new(".");
        let text = "line1\nline2\nline3\nline4\nline5";
        let result = d.first_lines(text, 2);
        assert_eq!(result, "line1\nline2");
    }
}
