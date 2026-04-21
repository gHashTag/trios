//! `tri dash` — Dashboard operations (sync, refresh)
//!
//! Usage:
//!   tri dash sync
//!   tri dash refresh

use anyhow::Result;
use std::process::Command;

use crate::gh::GhClient;

/// Sync dashboard with GitHub issue #143
pub fn dash_sync() -> Result<()> {
    println!("Syncing dashboard with GitHub...");

    let body = GhClient::issue_body(143)?;

    let dashboard_path = ".trinity/dashboard.md";
    std::fs::create_dir_all(".trinity")?;
    std::fs::write(dashboard_path, &body)?;

    let line_count = body.lines().count();
    println!(
        "Synced {} lines from #143 -> {}",
        line_count, dashboard_path
    );

    Ok(())
}

/// Refresh dashboard metrics by running live checks
pub fn dash_refresh() -> Result<()> {
    println!("Refreshing dashboard metrics...");

    let mut metrics = Vec::new();

    let test_output = Command::new("cargo")
        .args(["test", "--workspace", "--", "-q"])
        .output()
        .ok();
    if let Some(out) = test_output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let passed = count_keyword(&stdout, "passed");
        let failed = count_keyword(&stdout, "failed");
        metrics.push(format!("tests: {} passed, {} failed", passed, failed));
    }

    let issue_count = GhClient::list_agent_issues("")
        .map(|issues| issues.len())
        .unwrap_or(0);
    metrics.push(format!("open issues: {}", issue_count));

    let crate_count = std::fs::read_dir("crates").map(|d| d.count()).unwrap_or(0);
    metrics.push(format!("crates: {}", crate_count));

    let report: String = metrics.iter().map(|m| format!("- {}\n", m)).collect();
    let dashboard_path = ".trinity/dashboard-metrics.txt";
    std::fs::write(dashboard_path, &report)?;

    println!("{}", report);
    println!("Metrics written to {}", dashboard_path);

    Ok(())
}

fn count_keyword(stdout: &str, keyword: &str) -> i64 {
    stdout
        .lines()
        .filter(|l| l.contains("test result:"))
        .filter_map(|l| {
            let before = l.split(keyword).next()?;
            let num = before.trim().rsplit(' ').next()?;
            num.parse::<i64>().ok()
        })
        .sum()
}
