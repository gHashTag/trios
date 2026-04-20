pub mod check;
pub mod heal;
pub mod report;

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Diagnosis {
    pub crate_name: String,
    pub checks: Vec<CheckResult>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub fix: Option<String>,
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

    pub fn run_all(&self) -> Vec<Diagnosis> {
        let crates = self.discover_crates();
        let mut diagnoses = Vec::new();

        for crate_path in &crates {
            let crate_name = crate_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let checks = vec![
                self.check_compiles(crate_path, &crate_name),
                self.check_tests(crate_path, &crate_name),
                self.check_clippy(crate_path, &crate_name),
                self.check_has_lib_or_bin(crate_path, &crate_name),
            ];

            diagnoses.push(Diagnosis { crate_name, checks });
        }

        diagnoses
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

    fn check_compiles(&self, _crate_path: &Path, name: &str) -> CheckResult {
        let output = Command::new("cargo")
            .args(["check", "-p", name])
            .current_dir(&self.workspace_root)
            .output();

        match output {
            Ok(out) if out.status.success() => CheckResult {
                name: "compiles".into(),
                status: CheckStatus::Green,
                message: "OK".into(),
                fix: None,
            },
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                CheckResult {
                    name: "compiles".into(),
                    status: CheckStatus::Red,
                    message: stderr.chars().take(500).collect(),
                    fix: Some("Run `cargo check -p <crate>` and fix errors".into()),
                }
            }
            Err(e) => CheckResult {
                name: "compiles".into(),
                status: CheckStatus::Red,
                message: format!("Failed to run cargo: {}", e),
                fix: None,
            },
        }
    }

    fn check_tests(&self, _crate_path: &Path, name: &str) -> CheckResult {
        let output = Command::new("cargo")
            .args(["test", "-p", name, "--", "--test-threads=1"])
            .current_dir(&self.workspace_root)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let passed = stdout
                    .lines()
                    .find(|l| l.contains("test result:"))
                    .unwrap_or("")
                    .to_string();
                CheckResult {
                    name: "tests".into(),
                    status: CheckStatus::Green,
                    message: passed,
                    fix: None,
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                CheckResult {
                    name: "tests".into(),
                    status: CheckStatus::Red,
                    message: stderr.chars().take(500).collect(),
                    fix: Some("Run `cargo test -p <crate>` and fix failures".into()),
                }
            }
            Err(e) => CheckResult {
                name: "tests".into(),
                status: CheckStatus::Red,
                message: format!("Failed to run tests: {}", e),
                fix: None,
            },
        }
    }

    fn check_clippy(&self, _crate_path: &Path, name: &str) -> CheckResult {
        let output = Command::new("cargo")
            .args(["clippy", "-p", name, "--", "-D", "warnings"])
            .current_dir(&self.workspace_root)
            .output();

        match output {
            Ok(out) if out.status.success() => CheckResult {
                name: "clippy".into(),
                status: CheckStatus::Green,
                message: "No warnings".into(),
                fix: None,
            },
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                CheckResult {
                    name: "clippy".into(),
                    status: CheckStatus::Yellow,
                    message: stderr.chars().take(500).collect(),
                    fix: Some("Run `cargo clippy -p <crate> -- -D warnings` and fix".into()),
                }
            }
            Err(e) => CheckResult {
                name: "clippy".into(),
                status: CheckStatus::Red,
                message: format!("Failed to run clippy: {}", e),
                fix: None,
            },
        }
    }

    fn check_has_lib_or_bin(&self, crate_path: &Path, _name: &str) -> CheckResult {
        let has_lib = crate_path.join("src/lib.rs").exists();
        let has_main = crate_path.join("src/main.rs").exists();

        if has_lib || has_main {
            CheckResult {
                name: "entry_point".into(),
                status: CheckStatus::Green,
                message: if has_lib {
                    "lib.rs present"
                } else {
                    "main.rs present"
                }
                .into(),
                fix: None,
            }
        } else {
            CheckResult {
                name: "entry_point".into(),
                status: CheckStatus::Red,
                message: "No src/lib.rs or src/main.rs found".into(),
                fix: Some("Create src/lib.rs or src/main.rs".into()),
            }
        }
    }
}
