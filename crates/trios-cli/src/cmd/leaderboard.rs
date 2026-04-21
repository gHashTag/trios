//! `tri leaderboard` — Show experiment leaderboard
//!
//! Usage:
//!   tri leaderboard
//!   tri leaderboard --agent FOXTROT

use anyhow::{Context, Result};

use crate::db::{Entry, Leaderboard};

/// Show leaderboard
pub fn leaderboard_show(agent_filter: Option<&str>) -> Result<()> {
    println!("🏆 Leaderboard");
    println!();

    let ldb = Leaderboard::open()
        .context("Failed to open leaderboard")?;

    let stats = ldb.stats()
        .context("Failed to get stats")?;

    println!("Total entries: {}", stats.count);
    println!("Best val_bpb: {:.4}", stats.min_bpb);
    println!("Average val_bpb: {:.4}", stats.avg_bpb);
    println!();

    let entries = if let Some(agent) = agent_filter {
        ldb.by_agent(agent)?
    } else {
        ldb.top(20)?
    };

    if entries.is_empty() {
        println!("No entries yet");
        return Ok(());
    }

    println!("| Rank | Agent | Exp ID | Val BPB | Time |");
    println!("|------|-------|--------|---------|------|");

    for (i, entry) in entries.iter().enumerate() {
        println!(
            "| {} | {} | {} | {:.4} | {:.1}s |",
            i + 1,
            entry.agent,
            entry.exp_id,
            entry.val_bpb,
            entry.time_sec
        );
    }

    Ok(())
}

/// Export leaderboard as JSON
pub fn leaderboard_export() -> Result<String> {
    let ldb = Leaderboard::open()?;
    let entries = ldb.top(1000)?;

    serde_json::to_string_pretty(&entries)
        .context("Failed to serialize leaderboard")
}
