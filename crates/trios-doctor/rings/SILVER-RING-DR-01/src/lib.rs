//! SILVER-RING-DR-01 — Check runner
//! Executes cargo check / clippy / test / fmt and returns WorkspaceCheck results.
//! Per-crate granularity + ring doc verification (7 checks per ring).

use std::path::{Path, PathBuf};
use std::process::Command;
use trios_doctor_dr00::{CheckStatus, WorkspaceCheck, WorkspaceDiagnosis};

pub struct Doctor {
    pub workspace_root: PathBuf,
}

impl Doctor {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
        }
    }

    /// Discover all crates in workspace (directories with Cargo.toml under crates/)
    pub fn discover_crates(&self) -> Vec<PathBuf> {
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

    /// Discover all ring directories (directories with Cargo.toml under rings/)
    pub fn discover_rings(&self, crate_path: &Path) -> Vec<PathBuf> {
        let rings_dir = crate_path.join("rings");
        let mut result = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&rings_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("Cargo.toml").exists() {
                    result.push(path);
                }
            }
        }
        result.sort();
        result
    }

    pub fn count_crates(&self) -> usize {
        self.discover_crates().len()
    }

    /// Run all workspace-level checks
    pub fn run_all(&self) -> WorkspaceDiagnosis {
        let mut checks = vec![
            self.workspace_check(),
            self.workspace_clippy(),
            self.workspace_test(),
            self.check_fmt(),
            self.ring_structure_check(),
        ];

        // Add per-crate ring doc checks
        for crate_path in self.discover_crates() {
            let crate_name = crate_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            for ring_path in self.discover_rings(&crate_path) {
                checks.push(self.check_ring_docs(&ring_path, &crate_name));
            }
        }

        WorkspaceDiagnosis {
            workspace: self.workspace_root.display().to_string(),
            crate_count: self.count_crates(),
            checks,
        }
    }

    // ── Workspace-level checks ──────────────────────────────────────

    pub fn workspace_check(&self) -> WorkspaceCheck {
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

    pub fn workspace_test(&self) -> WorkspaceCheck {
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

    pub fn workspace_clippy(&self) -> WorkspaceCheck {
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

    /// `cargo fmt --check --workspace` — formatting verification
    pub fn check_fmt(&self) -> WorkspaceCheck {
        let output = Command::new("cargo")
            .args(["fmt", "--check", "--all"])
            .current_dir(&self.workspace_root)
            .output();

        match output {
            Ok(out) if out.status.success() => WorkspaceCheck {
                name: "cargo fmt --check".into(),
                status: CheckStatus::Green,
                message: "All files formatted".into(),
                failed_crates: vec![],
            },
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                WorkspaceCheck {
                    name: "cargo fmt --check".into(),
                    status: CheckStatus::Yellow,
                    message: self.first_lines(&stdout, 10),
                    failed_crates: vec![],
                }
            }
            Err(e) => WorkspaceCheck {
                name: "cargo fmt --check".into(),
                status: CheckStatus::Red,
                message: format!("cargo fmt failed: {}", e),
                failed_crates: vec![],
            },
        }
    }

    /// L-ARCH-001: verify no `src/` at crate root level — only `rings/` allowed
    pub fn ring_structure_check(&self) -> WorkspaceCheck {
        let mut violations: Vec<String> = Vec::new();

        for crate_path in self.discover_crates() {
            let rings_dir = crate_path.join("rings");
            if !rings_dir.exists() {
                continue; // no rings/ = not a ring-architecture crate
            }
            let src_dir = crate_path.join("src");
            if src_dir.exists() {
                let name = crate_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                violations.push(format!(
                    "{}: has src/ alongside rings/ (L-ARCH-001 violation)",
                    name
                ));
            }
        }

        if violations.is_empty() {
            WorkspaceCheck {
                name: "ring-structure (L-ARCH-001)".into(),
                status: CheckStatus::Green,
                message: "All ring-architecture crates compliant".into(),
                failed_crates: vec![],
            }
        } else {
            WorkspaceCheck {
                name: "ring-structure (L-ARCH-001)".into(),
                status: CheckStatus::Yellow,
                message: violations.join("\n"),
                failed_crates: violations
                    .iter()
                    .map(|v| v.split(':').next().unwrap_or("").to_string())
                    .collect(),
            }
        }
    }

    /// Check that a ring directory has RING.md, AGENTS.md, TASK.md
    pub fn check_ring_docs(&self, ring_path: &Path, crate_name: &str) -> WorkspaceCheck {
        let ring_name = ring_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let check_name = format!("ring-docs: {}/{}", crate_name, ring_name);

        let required = ["RING.md", "AGENTS.md", "TASK.md"];
        let mut missing: Vec<String> = Vec::new();

        for doc in &required {
            let path = ring_path.join(doc);
            if !path.exists() {
                missing.push(doc.to_string());
            } else if let Ok(content) = std::fs::read_to_string(&path) {
                if content.trim().is_empty() {
                    missing.push(format!("{} (empty)", doc));
                }
            }
        }

        if missing.is_empty() {
            WorkspaceCheck {
                name: check_name,
                status: CheckStatus::Green,
                message: "All ring docs present".into(),
                failed_crates: vec![],
            }
        } else {
            WorkspaceCheck {
                name: check_name,
                status: CheckStatus::Yellow,
                message: format!("Missing: {}", missing.join(", ")),
                failed_crates: vec![ring_name],
            }
        }
    }

    // ── Per-crate checks ────────────────────────────────────────────

    pub fn check_crate(&self, crate_path: &Path) -> WorkspaceCheck {
        let name = crate_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let output = Command::new("cargo")
            .args(["check", "-p", &name])
            .current_dir(&self.workspace_root)
            .output();

        match output {
            Ok(out) if out.status.success() => WorkspaceCheck {
                name: format!("cargo check -p {}", name),
                status: CheckStatus::Green,
                message: "compiles".into(),
                failed_crates: vec![],
            },
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                WorkspaceCheck {
                    name: format!("cargo check -p {}", name),
                    status: CheckStatus::Red,
                    message: self.first_lines(&stderr, 10),
                    failed_crates: vec![name],
                }
            }
            Err(e) => WorkspaceCheck {
                name: format!("cargo check -p {}", name),
                status: CheckStatus::Red,
                message: format!("failed: {}", e),
                failed_crates: vec![name],
            },
        }
    }

    pub fn clippy_crate(&self, crate_path: &Path) -> WorkspaceCheck {
        let name = crate_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let output = Command::new("cargo")
            .args(["clippy", "-p", &name, "--", "-D", "warnings"])
            .current_dir(&self.workspace_root)
            .output();

        match output {
            Ok(out) if out.status.success() => WorkspaceCheck {
                name: format!("cargo clippy -p {}", name),
                status: CheckStatus::Green,
                message: "0 warnings".into(),
                failed_crates: vec![],
            },
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                WorkspaceCheck {
                    name: format!("cargo clippy -p {}", name),
                    status: CheckStatus::Yellow,
                    message: self.first_lines(&stderr, 10),
                    failed_crates: vec![name],
                }
            }
            Err(e) => WorkspaceCheck {
                name: format!("cargo clippy -p {}", name),
                status: CheckStatus::Red,
                message: format!("clippy failed: {}", e),
                failed_crates: vec![name],
            },
        }
    }

    pub fn test_crate(&self, crate_path: &Path) -> WorkspaceCheck {
        let name = crate_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let output = Command::new("cargo")
            .args(["test", "-p", &name])
            .current_dir(&self.workspace_root)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                WorkspaceCheck {
                    name: format!("cargo test -p {}", name),
                    status: CheckStatus::Green,
                    message: self.extract_test_summary(&stdout),
                    failed_crates: vec![],
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                WorkspaceCheck {
                    name: format!("cargo test -p {}", name),
                    status: CheckStatus::Yellow,
                    message: self.first_lines(&stderr, 10),
                    failed_crates: vec![name],
                }
            }
            Err(e) => WorkspaceCheck {
                name: format!("cargo test -p {}", name),
                status: CheckStatus::Red,
                message: format!("test failed: {}", e),
                failed_crates: vec![name],
            },
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────

    fn extract_failed_crates(&self, output: &str) -> Vec<String> {
        let mut crates = Vec::new();
        for line in output.lines() {
            if line.contains("error") && line.contains("crates/") {
                for part in line.split_whitespace() {
                    if part.starts_with("crates/") {
                        if let Some(name) = part.split('/').nth(1) {
                            let name = name.trim_end_matches(':');
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

    #[test]
    fn count_crates_finds_cargo_toml() {
        let tmp = tempfile::TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crates").join("fake-crate");
        std::fs::create_dir_all(&crate_dir).unwrap();
        std::fs::write(crate_dir.join("Cargo.toml"), "").unwrap();
        let d = Doctor::new(tmp.path());
        assert_eq!(d.count_crates(), 1);
    }

    #[test]
    fn discover_rings_finds_ring_dirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ring_dir = tmp.path().join("rings").join("SILVER-RING-00");
        std::fs::create_dir_all(&ring_dir).unwrap();
        std::fs::write(ring_dir.join("Cargo.toml"), "").unwrap();
        let d = Doctor::new(tmp.path());
        let rings = d.discover_rings(tmp.path());
        assert_eq!(rings.len(), 1);
    }

    #[test]
    fn check_ring_docs_detects_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ring_dir = tmp.path().join("SILVER-RING-00");
        std::fs::create_dir_all(&ring_dir).unwrap();
        // No docs created
        let d = Doctor::new(tmp.path());
        let check = d.check_ring_docs(&ring_dir, "test-crate");
        assert_eq!(check.status, CheckStatus::Yellow);
        assert!(check.message.contains("RING.md"));
    }

    #[test]
    fn check_ring_docs_passes_when_present() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ring_dir = tmp.path().join("SILVER-RING-00");
        std::fs::create_dir_all(&ring_dir).unwrap();
        std::fs::write(ring_dir.join("RING.md"), "# RING").unwrap();
        std::fs::write(ring_dir.join("AGENTS.md"), "# AGENTS").unwrap();
        std::fs::write(ring_dir.join("TASK.md"), "# TASK").unwrap();
        let d = Doctor::new(tmp.path());
        let check = d.check_ring_docs(&ring_dir, "test-crate");
        assert_eq!(check.status, CheckStatus::Green);
    }

    #[test]
    fn ring_structure_detects_violation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let crate_dir = tmp.path().join("crates").join("test-crate");
        let rings_dir = crate_dir.join("rings").join("SILVER-RING-00");
        let src_dir = crate_dir.join("src");
        std::fs::create_dir_all(&rings_dir).unwrap();
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(crate_dir.join("Cargo.toml"), "").unwrap();
        let d = Doctor::new(tmp.path());
        let check = d.ring_structure_check();
        assert_eq!(check.status, CheckStatus::Yellow);
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
}
