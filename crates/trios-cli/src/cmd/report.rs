//! `tri report` — Auto-sync results to issue #143 table
//!
//! Usage:
//!   tri report ALFA "complete" --bpb 6.5609
//!   tri report FOXTROT "running"

use anyhow::{Context, Result};

use crate::{
    config::Config,
    gh::GhClient,
    lock::LockGuard,
    table::{parse_table, update_table},
};

/// Report experiment result to issue #143
pub fn report(agent: &str, status: &str, bpb: Option<f64>) -> Result<()> {
    println!("Reporting {}={} --bpb={:?}", agent, status, bpb);

    let _config = Config::load();

    let _lock = LockGuard::acquire().context("Failed to acquire lock for issue #143")?;

    let issue_num = 143;
    let body = GhClient::issue_body(issue_num).context("Failed to fetch issue #143")?;

    let rows = parse_table(&body, "Agent").context("Failed to parse #143 IGLA table")?;

    let _existing = rows.iter().find(|r| r.contains(agent));

    let updated_body =
        update_table(&body, agent, status, bpb).context("Failed to update #143 table")?;

    GhClient::issue_edit(issue_num, &updated_body).context("Failed to update issue #143")?;

    println!("Reported {}={} to #143", agent, status);

    Ok(())
}

/// Batch report multiple results
pub fn report_batch(results: Vec<(String, String, Option<f64>)>) -> Result<()> {
    let count = results.len();
    println!("Batch reporting {} results", count);

    let _lock = LockGuard::acquire()?;

    let issue_num = 143;
    let mut body = GhClient::issue_body(issue_num)?;

    for (agent, status, bpb) in results {
        body = update_table(&body, &agent, &status, bpb)?;
    }

    GhClient::issue_edit(issue_num, &body)?;

    println!("Batch reported {} results to #143", count);

    Ok(())
}
