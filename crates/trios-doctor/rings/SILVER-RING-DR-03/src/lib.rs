//! SILVER-RING-DR-03 — Report ring
//! Formats WorkspaceDiagnosis into human-readable, JSON, SARIF, and GitHub Actions output.

use trios_doctor_dr00::{CheckStatus, WorkspaceDiagnosis};

// ANSI color codes (no external dependency)
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

pub struct Reporter;

impl Reporter {
    /// Human-readable output with ANSI colors (for terminal)
    pub fn report_human(diagnosis: &WorkspaceDiagnosis) -> String {
        let mut out = String::new();
        out.push_str(&format!("{}=== trios-doctor ==={}\n", BOLD, RESET));
        out.push_str(&format!("Workspace: {}\n", diagnosis.workspace));
        out.push_str(&format!("Crates:    {}\n\n", diagnosis.crate_count));

        for check in &diagnosis.checks {
            let (color, icon, label) = match check.status {
                CheckStatus::Green => (GREEN, "OK", "GREEN"),
                CheckStatus::Yellow => (YELLOW, "WARN", "YELLOW"),
                CheckStatus::Red => (RED, "FAIL", "RED"),
            };
            out.push_str(&format!(
                "{}[{}] {} {}{}\n",
                color, icon, label, check.name, RESET
            ));
            if check.status != CheckStatus::Green {
                for line in check.message.lines().take(5) {
                    out.push_str(&format!("    {}\n", line));
                }
                if !check.failed_crates.is_empty() {
                    out.push_str(&format!(
                        "    Affected: {}\n",
                        check.failed_crates.join(", ")
                    ));
                }
            } else {
                out.push_str(&format!("    {}\n", check.message));
            }
            out.push('\n');
        }

        let status = Self::overall_status(diagnosis);
        let (color, summary) = match status {
            CheckStatus::Green => (GREEN, Self::summary_line(diagnosis)),
            CheckStatus::Yellow => (YELLOW, Self::summary_line(diagnosis)),
            CheckStatus::Red => (RED, Self::summary_line(diagnosis)),
        };
        out.push_str(&format!("{}{}{}\n", color, summary, RESET));

        out
    }

    /// Print human-readable output to stdout
    pub fn print_text(diagnosis: &WorkspaceDiagnosis) {
        println!("{}", Self::report_human(diagnosis));
    }

    /// Pretty-printed JSON output
    pub fn report_json(diagnosis: &WorkspaceDiagnosis) -> String {
        serde_json::to_string_pretty(diagnosis).unwrap_or_default()
    }

    /// Print JSON output to stdout
    pub fn print_json(diagnosis: &WorkspaceDiagnosis) {
        println!("{}", Self::report_json(diagnosis));
    }

    /// One-liner summary for CI logs
    pub fn summary_line(diagnosis: &WorkspaceDiagnosis) -> String {
        let green = diagnosis
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Green)
            .count();
        let total = diagnosis.checks.len();
        let all_ok = green == total;
        if all_ok {
            format!(
                "✅ {}/{} checks passed — {} crates",
                green, total, diagnosis.crate_count
            )
        } else {
            format!(
                "❌ {}/{} checks passed — {} crates",
                green, total, diagnosis.crate_count
            )
        }
    }

    /// Overall status: worst case wins
    pub fn overall_status(diagnosis: &WorkspaceDiagnosis) -> CheckStatus {
        let mut worst = CheckStatus::Green;
        for check in &diagnosis.checks {
            match check.status {
                CheckStatus::Red => return CheckStatus::Red,
                CheckStatus::Yellow => worst = CheckStatus::Yellow,
                CheckStatus::Green => {}
            }
        }
        worst
    }

    /// SARIF output format (Static Analysis Results Format for GitHub Code Scanning)
    pub fn report_sarif(diagnosis: &WorkspaceDiagnosis) -> String {
        let mut results = Vec::new();

        for check in &diagnosis.checks {
            if check.status == CheckStatus::Green {
                continue;
            }
            let level = match check.status {
                CheckStatus::Red => "error",
                CheckStatus::Yellow => "warning",
                CheckStatus::Green => unreachable!(),
            };
            let message = check.message.lines().next().unwrap_or("No message");
            let rule_id = check.name.replace('"', "\\\"");
            let msg_escaped = message.replace('"', "\\\"");

            results.push(format!(
                "    {{\n      \"ruleId\": \"{}\",\n      \"level\": \"{}\",\n      \"message\": {{\n        \"text\": \"{}\"\n      }}\n    }}",
                rule_id, level, msg_escaped
            ));
        }

        let results_json = results.join(",\n");

        // Build SARIF JSON manually to avoid format! brace escaping issues
        let mut sarif = String::new();
        sarif.push_str("{\n");
        sarif.push_str("  \"$schema\": \"https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json\",\n");
        sarif.push_str("  \"version\": \"2.1.0\",\n");
        sarif.push_str("  \"runs\": [\n");
        sarif.push_str("    {\n");
        sarif.push_str("      \"tool\": {\n");
        sarif.push_str("        \"driver\": {\n");
        sarif.push_str("          \"name\": \"trios-doctor\",\n");
        sarif.push_str("          \"version\": \"0.1.0\",\n");
        sarif.push_str("          \"informationUri\": \"https://github.com/gHashTag/trios\"\n");
        sarif.push_str("        }\n");
        sarif.push_str("      },\n");
        sarif.push_str("      \"results\": [\n");
        sarif.push_str(&results_json);
        sarif.push_str("\n      ]\n");
        sarif.push_str("    }\n");
        sarif.push_str("  ]\n");
        sarif.push_str("}");
        sarif
    }

    /// GitHub Actions format (::error / ::warning annotations)
    pub fn report_github(diagnosis: &WorkspaceDiagnosis) -> String {
        let mut out = Vec::new();

        for check in &diagnosis.checks {
            if check.status == CheckStatus::Green {
                continue;
            }
            let level = match check.status {
                CheckStatus::Red => "error",
                CheckStatus::Yellow => "warning",
                CheckStatus::Green => continue,
            };
            let message = check.message.lines().next().unwrap_or("No message");
            out.push(format!(
                "::{} title={}:{}",
                level,
                check.name.replace(',', "\\,"),
                message
            ));
        }

        out.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trios_doctor_dr00::{CheckStatus, WorkspaceCheck, WorkspaceDiagnosis};

    fn make_diag(statuses: Vec<CheckStatus>) -> WorkspaceDiagnosis {
        WorkspaceDiagnosis {
            workspace: "/tmp".into(),
            crate_count: 3,
            checks: statuses
                .into_iter()
                .enumerate()
                .map(|(i, s)| WorkspaceCheck {
                    name: format!("test check {}", i),
                    status: s,
                    message: "ok".into(),
                    failed_crates: vec![],
                })
                .collect(),
        }
    }

    #[test]
    fn summary_line_all_green() {
        let diag = make_diag(vec![CheckStatus::Green]);
        let s = Reporter::summary_line(&diag);
        assert!(s.contains("✅"));
        assert!(s.contains("1/1"));
    }

    #[test]
    fn summary_line_red() {
        let diag = make_diag(vec![CheckStatus::Red]);
        let s = Reporter::summary_line(&diag);
        assert!(s.contains("❌"));
    }

    #[test]
    fn overall_status_green() {
        let diag = make_diag(vec![CheckStatus::Green, CheckStatus::Green]);
        assert_eq!(Reporter::overall_status(&diag), CheckStatus::Green);
    }

    #[test]
    fn overall_status_yellow() {
        let diag = make_diag(vec![CheckStatus::Green, CheckStatus::Yellow]);
        assert_eq!(Reporter::overall_status(&diag), CheckStatus::Yellow);
    }

    #[test]
    fn overall_status_red() {
        let diag = make_diag(vec![CheckStatus::Green, CheckStatus::Red]);
        assert_eq!(Reporter::overall_status(&diag), CheckStatus::Red);
    }

    #[test]
    fn report_human_contains_workspace() {
        let diag = make_diag(vec![CheckStatus::Green]);
        let output = Reporter::report_human(&diag);
        assert!(output.contains("/tmp"));
        assert!(output.contains("trios-doctor"));
    }

    #[test]
    fn report_human_uses_ansi_colors() {
        let diag = make_diag(vec![CheckStatus::Red]);
        let output = Reporter::report_human(&diag);
        assert!(output.contains("\x1b[31m")); // RED
    }

    #[test]
    fn report_json_is_valid() {
        let diag = make_diag(vec![CheckStatus::Green]);
        let json = Reporter::report_json(&diag);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["workspace"], "/tmp");
    }

    #[test]
    fn report_sarif_is_valid() {
        let diag = make_diag(vec![CheckStatus::Yellow]);
        let sarif = Reporter::report_sarif(&diag);
        assert!(sarif.contains("\"version\": \"2.1.0\""));
        assert!(sarif.contains("trios-doctor"));
        assert!(sarif.contains("warning"));
    }

    #[test]
    fn report_github_format() {
        let diag = make_diag(vec![CheckStatus::Yellow, CheckStatus::Red]);
        let gh = Reporter::report_github(&diag);
        assert!(gh.contains("::warning"));
        assert!(gh.contains("::error"));
    }

    #[test]
    fn report_sarif_skips_green() {
        let diag = make_diag(vec![CheckStatus::Green]);
        let sarif = Reporter::report_sarif(&diag);
        assert!(!sarif.contains("warning"));
        assert!(!sarif.contains("error"));
    }
}
