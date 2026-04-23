//! SILVER-RING-DR-02 — Heal ring
//! Auto-fix and repair logic for trios workspace.

use trios_doctor_dr00::{CheckStatus, WorkspaceDiagnosis};

#[derive(Debug, Clone)]
pub struct HealResult {
    pub fixed: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: Vec<String>,
}

pub struct Healer {
    workspace_root: std::path::PathBuf,
    pub dry_run: bool,
}

impl Healer {
    pub fn new(workspace_root: impl AsRef<std::path::Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
            dry_run: true,
        }
    }

    pub fn heal(&self, diagnosis: &WorkspaceDiagnosis) -> HealResult {
        let mut result = HealResult {
            fixed: vec![],
            skipped: vec![],
            failed: vec![],
        };

        for check in &diagnosis.checks {
            match check.status {
                CheckStatus::Green => {}
                CheckStatus::Yellow => {
                    // Yellow = clippy warnings — attempt cargo fix
                    if self.dry_run {
                        result.skipped.push(format!("[dry-run] would fix: {}", check.name));
                    } else {
                        result.skipped.push(format!("TODO: auto-fix {}", check.name));
                    }
                }
                CheckStatus::Red => {
                    result.skipped.push(format!("Manual fix required: {}", check.name));
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trios_doctor_dr00::{CheckStatus, WorkspaceCheck, WorkspaceDiagnosis};

    #[test]
    fn heal_dry_run_skips_all() {
        let healer = Healer::new("/tmp");
        assert!(healer.dry_run);
        let diag = WorkspaceDiagnosis {
            workspace: "/tmp".into(),
            crate_count: 1,
            checks: vec![WorkspaceCheck {
                name: "clippy".into(),
                status: CheckStatus::Yellow,
                message: "warning: unused".into(),
                failed_crates: vec![],
            }],
        };
        let result = healer.heal(&diag);
        assert!(!result.skipped.is_empty());
        assert!(result.fixed.is_empty());
    }
}
