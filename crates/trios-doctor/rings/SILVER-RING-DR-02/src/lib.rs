//! SILVER-RING-DR-02 — Heal ring
//! Auto-fix and repair logic for trios workspace.
//! Only heals Yellow checks. Red checks require manual intervention.

use std::path::PathBuf;
use std::process::Command;
use trios_doctor_dr00::{CheckStatus, WorkspaceCheck, WorkspaceDiagnosis};

#[derive(Debug, Clone)]
pub struct HealResult {
    pub fixed: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HealEntry {
    pub check_name: String,
    pub action: String,
    pub success: bool,
}

pub struct Healer {
    pub workspace_root: PathBuf,
    pub dry_run: bool,
}

impl Healer {
    pub fn new(workspace_root: impl AsRef<std::path::Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
            dry_run: true,
        }
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Heal all checks in a diagnosis. Only Yellow checks are auto-fixed.
    pub fn heal(&self, diagnosis: &WorkspaceDiagnosis) -> HealResult {
        let mut result = HealResult {
            fixed: vec![],
            skipped: vec![],
            failed: vec![],
        };

        for check in &diagnosis.checks {
            match check.status {
                CheckStatus::Green => {
                    // Nothing to do
                }
                CheckStatus::Yellow => {
                    let entry = self.heal_check(check);
                    if entry.success {
                        result.fixed.push(format!("{}: {}", entry.check_name, entry.action));
                    } else {
                        result.failed.push(format!("{}: {}", entry.check_name, entry.action));
                    }
                }
                CheckStatus::Red => {
                    result.skipped.push(format!("Manual fix required: {}", check.name));
                }
            }
        }

        result
    }

    /// Heal a single check based on its name/status
    pub fn heal_check(&self, check: &WorkspaceCheck) -> HealEntry {
        let name = &check.name;

        if name.starts_with("cargo fmt") {
            self.heal_fmt()
        } else if name.starts_with("cargo clippy") {
            self.heal_clippy()
        } else if name.starts_with("ring-docs:") {
            self.heal_ring_docs(check)
        } else if name.starts_with("ring-structure") {
            self.heal_ring_structure(check)
        } else {
            HealEntry {
                check_name: name.clone(),
                action: "No auto-fix available".into(),
                success: false,
            }
        }
    }

    /// `cargo fmt --all` — auto-format all code
    fn heal_fmt(&self) -> HealEntry {
        if self.dry_run {
            return HealEntry {
                check_name: "cargo fmt".into(),
                action: "[dry-run] would run cargo fmt --all".into(),
                success: true,
            };
        }

        let output = Command::new("cargo")
            .args(["fmt", "--all"])
            .current_dir(&self.workspace_root)
            .output();

        match output {
            Ok(out) if out.status.success() => HealEntry {
                check_name: "cargo fmt".into(),
                action: "Formatted all code".into(),
                success: true,
            },
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                HealEntry {
                    check_name: "cargo fmt".into(),
                    action: format!("fmt failed: {}", stderr.lines().next().unwrap_or("")),
                    success: false,
                }
            }
            Err(e) => HealEntry {
                check_name: "cargo fmt".into(),
                action: format!("fmt failed: {}", e),
                success: false,
            },
        }
    }

    /// `cargo fix --allow-dirty --allow-staged` — auto-fix clippy warnings
    fn heal_clippy(&self) -> HealEntry {
        if self.dry_run {
            return HealEntry {
                check_name: "cargo clippy".into(),
                action: "[dry-run] would run cargo fix --allow-dirty --allow-staged".into(),
                success: true,
            };
        }

        let output = Command::new("cargo")
            .args(["fix", "--allow-dirty", "--allow-staged", "--workspace"])
            .current_dir(&self.workspace_root)
            .output();

        match output {
            Ok(out) if out.status.success() => HealEntry {
                check_name: "cargo clippy".into(),
                action: "Applied cargo fix".into(),
                success: true,
            },
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                HealEntry {
                    check_name: "cargo clippy".into(),
                    action: format!("fix failed: {}", stderr.lines().next().unwrap_or("")),
                    success: false,
                }
            }
            Err(e) => HealEntry {
                check_name: "cargo clippy".into(),
                action: format!("fix failed: {}", e),
                success: false,
            },
        }
    }

    /// Create missing RING.md / AGENTS.md / TASK.md template files
    fn heal_ring_docs(&self, check: &WorkspaceCheck) -> HealEntry {
        // Parse ring path from check name: "ring-docs: crate-name/RING-NAME"
        let parts: Vec<&str> = check.name.splitn(2, ": ").collect();
        if parts.len() < 2 {
            return HealEntry {
                check_name: check.name.clone(),
                action: "Cannot parse ring path".into(),
                success: false,
            };
        }

        let ring_id = parts[1]; // e.g. "trios-doctor/SILVER-RING-DR-00"
        let ring_parts: Vec<&str> = ring_id.split('/').collect();
        if ring_parts.len() < 2 {
            return HealEntry {
                check_name: check.name.clone(),
                action: "Cannot parse ring path".into(),
                success: false,
            };
        }

        let ring_path = self.workspace_root
            .join("crates")
            .join(ring_parts[0])
            .join("rings")
            .join(ring_parts[1]);

        if !ring_path.exists() {
            return HealEntry {
                check_name: check.name.clone(),
                action: format!("Ring directory not found: {}", ring_path.display()),
                success: false,
            };
        }

        if self.dry_run {
            return HealEntry {
                check_name: check.name.clone(),
                action: format!("[dry-run] would create missing docs in {}", ring_id),
                success: true,
            };
        }

        let mut created = Vec::new();

        // RING.md template
        let ring_md = ring_path.join("RING.md");
        if !ring_md.exists() {
            let template = format!(
                "# RING — {}\n\n## Identity\n\n| Field | Value |\n|-------|-------|\n| Metal | 🥈 Silver |\n| Crate | {} |\n| Sealed | No |\n\n## Purpose\n\nTODO: Describe ring purpose.\n\n## API Surface (pub)\n\nTODO: List public API.\n\n## Dependencies\n\nTODO: List dependencies.\n\n## Laws\n\n- R1: No imports from sibling rings\n- R2: Separate package in workspace\n- R3: This RING.md is required\n- L6: Pure Rust only\n",
                ring_parts[1], ring_parts[0]
            );
            if std::fs::write(&ring_md, template).is_ok() {
                created.push("RING.md");
            }
        }

        // AGENTS.md template
        let agents_md = ring_path.join("AGENTS.md");
        if !agents_md.exists() {
            let template = format!(
                "# AGENTS — {}\n\n## Agent Protocol\n\nThis ring follows the trios agent protocol.\n\n## Commands\n\n- `check` — verify ring health\n- `heal` — auto-fix issues\n- `report` — generate status report\n",
                ring_parts[1]
            );
            if std::fs::write(&agents_md, template).is_ok() {
                created.push("AGENTS.md");
            }
        }

        // TASK.md template
        let task_md = ring_path.join("TASK.md");
        if !task_md.exists() {
            let template = format!(
                "# TASK — {}\n\n## Status: ACTIVE\n\n## Completed\n\n- [x] Ring scaffolding created\n\n## Open\n\n- [ ] Implement ring logic\n\n## Blocked by\n\nNothing.\n",
                ring_parts[1]
            );
            if std::fs::write(&task_md, template).is_ok() {
                created.push("TASK.md");
            }
        }

        if created.is_empty() {
            HealEntry {
                check_name: check.name.clone(),
                action: "No missing docs to create".into(),
                success: true,
            }
        } else {
            HealEntry {
                check_name: check.name.clone(),
                action: format!("Created: {}", created.join(", ")),
                success: true,
            }
        }
    }

    /// L-ARCH-001 violation: src/ exists alongside rings/
    /// Cannot auto-fix (requires manual migration), but provides guidance
    fn heal_ring_structure(&self, check: &WorkspaceCheck) -> HealEntry {
        HealEntry {
            check_name: check.name.clone(),
            action: "L-ARCH-001: Manual migration required — move src/ contents to rings/ and delete src/".into(),
            success: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trios_doctor_dr00::{CheckStatus, WorkspaceCheck, WorkspaceDiagnosis};

    fn make_diag(statuses: Vec<CheckStatus>) -> WorkspaceDiagnosis {
        WorkspaceDiagnosis {
            workspace: "/tmp".into(),
            crate_count: 1,
            checks: statuses
                .into_iter()
                .enumerate()
                .map(|(i, s)| WorkspaceCheck {
                    name: format!("check-{}", i),
                    status: s,
                    message: "test".into(),
                    failed_crates: vec![],
                })
                .collect(),
        }
    }

    #[test]
    fn heal_dry_run_skips_all() {
        let healer = Healer::new("/tmp");
        assert!(healer.dry_run);
        let diag = WorkspaceDiagnosis {
            workspace: "/tmp".into(),
            crate_count: 1,
            checks: vec![WorkspaceCheck {
                name: "cargo clippy --workspace".into(),
                status: CheckStatus::Yellow,
                message: "warning: unused".into(),
                failed_crates: vec![],
            }],
        };
        let result = healer.heal(&diag);
        assert!(!result.fixed.is_empty()); // dry-run counts as "fixed" (would fix)
        assert!(result.failed.is_empty());
    }

    #[test]
    fn heal_skips_red_checks() {
        let healer = Healer::new("/tmp");
        let diag = make_diag(vec![CheckStatus::Red]);
        let result = healer.heal(&diag);
        assert!(result.skipped.len() == 1);
        assert!(result.fixed.is_empty());
    }

    #[test]
    fn heal_skips_green_checks() {
        let healer = Healer::new("/tmp");
        let diag = make_diag(vec![CheckStatus::Green]);
        let result = healer.heal(&diag);
        assert!(result.fixed.is_empty());
        assert!(result.skipped.is_empty());
        assert!(result.failed.is_empty());
    }

    #[test]
    fn heal_fmt_dry_run() {
        let healer = Healer::new("/tmp");
        let check = WorkspaceCheck {
            name: "cargo fmt --check".into(),
            status: CheckStatus::Yellow,
            message: "diff".into(),
            failed_crates: vec![],
        };
        let entry = healer.heal_check(&check);
        assert!(entry.success);
        assert!(entry.action.contains("dry-run"));
    }

    #[test]
    fn heal_ring_docs_creates_templates() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ring_dir = tmp.path()
            .join("crates")
            .join("test-crate")
            .join("rings")
            .join("SILVER-RING-00");
        std::fs::create_dir_all(&ring_dir).unwrap();

        let healer = Healer::new(tmp.path()).with_dry_run(false);
        let check = WorkspaceCheck {
            name: "ring-docs: test-crate/SILVER-RING-00".into(),
            status: CheckStatus::Yellow,
            message: "Missing: RING.md, AGENTS.md, TASK.md".into(),
            failed_crates: vec![],
        };
        let entry = healer.heal_check(&check);
        assert!(entry.success);
        assert!(ring_dir.join("RING.md").exists());
        assert!(ring_dir.join("AGENTS.md").exists());
        assert!(ring_dir.join("TASK.md").exists());
    }

    #[test]
    fn with_dry_run_builder() {
        let healer = Healer::new("/tmp").with_dry_run(false);
        assert!(!healer.dry_run);
    }
}
