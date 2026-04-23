//! SILVER-RING-DR-03 — Report ring
//! Formats WorkspaceDiagnosis into human-readable output.

use trios_doctor_dr00::{CheckStatus, WorkspaceDiagnosis};

pub struct Reporter;

impl Reporter {
    pub fn print_text(diagnosis: &WorkspaceDiagnosis) {
        println!("=== trios-doctor ===");
        println!("Workspace: {}", diagnosis.workspace);
        println!("Crates: {}", diagnosis.crate_count);
        println!();
        for check in &diagnosis.checks {
            let icon = match check.status {
                CheckStatus::Green => "✅",
                CheckStatus::Yellow => "⚠️",
                CheckStatus::Red => "❌",
            };
            println!("{} {}", icon, check.name);
            if check.status != CheckStatus::Green {
                println!("   {}", check.message.lines().next().unwrap_or(""));
            }
            if !check.failed_crates.is_empty() {
                println!("   Failed crates: {}", check.failed_crates.join(", "));
            }
        }
    }

    pub fn print_json(diagnosis: &WorkspaceDiagnosis) {
        println!("{}", serde_json::to_string_pretty(diagnosis).unwrap_or_default());
    }

    pub fn summary_line(diagnosis: &WorkspaceDiagnosis) -> String {
        let green = diagnosis.checks.iter().filter(|c| c.status == CheckStatus::Green).count();
        let total = diagnosis.checks.len();
        let all_ok = green == total;
        if all_ok {
            format!("✅ {}/{} checks passed — {} crates", green, total, diagnosis.crate_count)
        } else {
            format!("❌ {}/{} checks passed — {} crates", green, total, diagnosis.crate_count)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trios_doctor_dr00::{CheckStatus, WorkspaceCheck, WorkspaceDiagnosis};

    fn make_diag(status: CheckStatus) -> WorkspaceDiagnosis {
        WorkspaceDiagnosis {
            workspace: "/tmp".into(),
            crate_count: 3,
            checks: vec![WorkspaceCheck {
                name: "test check".into(),
                status,
                message: "ok".into(),
                failed_crates: vec![],
            }],
        }
    }

    #[test]
    fn summary_line_all_green() {
        let diag = make_diag(CheckStatus::Green);
        let s = Reporter::summary_line(&diag);
        assert!(s.contains("✅"));
        assert!(s.contains("1/1"));
    }

    #[test]
    fn summary_line_red() {
        let diag = make_diag(CheckStatus::Red);
        let s = Reporter::summary_line(&diag);
        assert!(s.contains("❌"));
    }
}
