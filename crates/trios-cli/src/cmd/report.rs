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
    table::{parse_table, update_table, TableRow},
};

/// Report experiment result to issue #143
pub fn report(agent: &str, status: &str, bpb: Option<f64>) -> Result<()> {
    println!("📊 Reporting {}={} --bpb={:?}", agent, status, bpb);

    let config = Config::load()?;
    let gh = GhClient::new();

    // Acquire lock for #143 table update
    let _lock = LockGuard::acquire()
        .context("Failed to acquire lock for issue #143")?;

    // Get current issue #143 body
    let issue_num = 143;
    let body = gh.issue_body(issue_num)
        .context("Failed to fetch issue #143")?;

    // Parse existing table
    let mut rows = parse_table(&body)
        .context("Failed to parse #143 table")?;

    // Create or update row for this agent
    let new_row = TableRow {
        agent: agent.to_string(),
        exp_id: format!("{}-exp", agent.to_lowercase()),
        config: "L10".to_string(), // CPU-only
        train_bpb: bpb.unwrap_or(0.0),
        val_bpb: bpb.unwrap_or(0.0),
        status: status.to_string(),
    };

    // Update table
    let updated_body = update_table(&body, &new_row)
        .context("Failed to update #143 table")?;

    // Write back to issue
    gh.issue_update(issue_num, &updated_body)
        .context("Failed to update issue #143")?;

    println!("✓ Reported {}={} to #143", agent, status);

    Ok(())
}

/// Batch report multiple results
pub fn report_batch(results: Vec<(String, String, Option<f64>)>) -> Result<()> {
    println!("📊 Batch reporting {} results", results.len());

    let gh = GhClient::new();
    let _lock = LockGuard::acquire()?;

    let issue_num = 143;
    let mut body = gh.issue_body(issue_num)?;

    for (agent, status, bpb) in results {
        let row = TableRow {
            agent: agent.clone(),
            exp_id: format!("{}-exp", agent.to_lowercase()),
            config: "L10".to_string(),
            train_bpb: bpb.unwrap_or(0.0),
            val_bpb: bpb.unwrap_or(0.0),
            status: status.clone(),
        };

        body = update_table(&body, &row)?;
    }

    gh.issue_update(issue_num, &body)?;

    println!("✓ Batch reported {} results to #143", results.len());

    Ok(())
}
