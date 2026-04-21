//! `tri roster` — Update agent roster
//!
//! Usage:
//!   tri roster ALFA "active"
//!   tri roster FOXTROT "busy"

use anyhow::{Context, Result};

use crate::gh::GhClient;

const ROSTER_ISSUE: u32 = 143;

/// Update agent roster status
pub fn roster_update(agent: &str, status: &str) -> Result<()> {
    println!("Updating {} agent status: {}", agent, status);

    let _config = crate::config::Config::load();

    let issues = GhClient::list_agent_issues(agent).context("Failed to search for agent issues")?;

    if let Some(roster_issue) = issues.iter().find(|i| {
        i.title.contains("roster") || i.title.contains("Roster") || i.title.contains("ONE SHOT")
    }) {
        println!("Found dashboard issue: #{}", roster_issue.number);

        let body = GhClient::issue_body(roster_issue.number)?;

        let updated = update_agent_status_in_body(&body, agent, status);

        GhClient::issue_edit(roster_issue.number, &updated)?;
        println!("Updated #{}: {} -> {}", roster_issue.number, agent, status);
    } else {
        let body = GhClient::issue_body(ROSTER_ISSUE)?;
        let updated = update_agent_status_in_body(&body, agent, status);
        GhClient::issue_edit(ROSTER_ISSUE, &updated)?;
        println!("Updated #{}: {} -> {}", ROSTER_ISSUE, agent, status);
    }

    Ok(())
}

fn update_agent_status_in_body(body: &str, agent: &str, status: &str) -> String {
    let mut lines: Vec<String> = body.lines().map(String::from).collect();

    for line in lines.iter_mut() {
        if line.contains(agent) && line.contains('|') {
            let cells: Vec<&str> = line
                .trim_start_matches('|')
                .trim_end_matches('|')
                .split('|')
                .map(|s| s.trim())
                .collect();

            if cells.len() >= 4 {
                let new_line = format!(
                    "| {:30} | {:20} | {:15} | {} |",
                    cells.first().unwrap_or(&""),
                    cells.get(1).unwrap_or(&""),
                    status,
                    cells[3..]
                        .iter()
                        .map(|s| format!(" {} |", s))
                        .collect::<String>()
                );
                *line = new_line;
            }
        }
    }

    lines.join("\n")
}

/// Show all agents
pub fn roster_show() -> Result<()> {
    println!("Agent Roster");
    println!();

    let nato_codes = [
        "ALFA", "BRAVO", "CHARLIE", "DELTA", "ECHO", "FOXTROT", "GOLF", "HOTEL", "INDIA",
        "JULIETT", "KILO", "LIMA", "MIKE", "NOVEMBER", "OSCAR",
    ];

    for nato in &nato_codes {
        println!("  {} - {}", nato, agent_name(nato));
    }

    Ok(())
}

fn agent_name(nato: &str) -> &'static str {
    match nato {
        "ALFA" => "GoldenRatio",
        "BRAVO" => "RustaceanPrime",
        "CHARLIE" => "PhiGolfer",
        "DELTA" => "QuantumSweeper",
        "ECHO" => "CompilerWhisperer",
        "FOXTROT" => "BrowserShaman",
        "GOLF" => "OPENCODE",
        "HOTEL" => "Doctor",
        "INDIA" => "BridgeBuilder",
        "JULIETT" => "Trainer",
        "KILO" => "Auditor",
        "LIMA" => "Deployer",
        "MIKE" => "Sweeper",
        "NOVEMBER" => "Verifier",
        "OSCAR" => "Orchestrator",
        _ => "UNKNOWN",
    }
}
