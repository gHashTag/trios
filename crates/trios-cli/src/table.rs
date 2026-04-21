//! Markdown table parsing and rendering for issue #143
//!
//! Issue #143 contains a markdown table tracking experiment results.
//! This module parses and updates that table atomically.

use anyhow::{Context, Result};
use std::fmt::Write;

/// Parsed row from #143 table
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    pub agent: String,
    pub exp_id: String,
    pub config: String,
    pub train_bpb: f64,
    pub val_bpb: f64,
    pub status: String,
}

impl TableRow {
    pub fn format_row(&self) -> String {
        format!(
            "| {} | {} | {} | {:.4} | {:.4} | {} |",
            self.agent, self.exp_id, self.config, self.train_bpb, self.val_bpb, self.status
        )
    }
}

/// Parse issue #143 body to extract table rows
pub fn parse_table(body: &str) -> Result<Vec<TableRow>> {
    let mut rows = Vec::new();

    for line in body.lines() {
        // Skip header and separator
        if line.starts_with('|') && !line.contains("---") {
            let parts: Vec<&str> = line.split('|').collect();

            if parts.len() >= 7 {
                let agent = parts[1].trim().to_string();
                let exp_id = parts[2].trim().to_string();
                let config = parts[3].trim().to_string();
                let train_bpb: f64 = parts[4]
                    .trim()
                    .parse()
                    .context("Failed to parse train_bpb")?;
                let val_bpb: f64 = parts[5]
                    .trim()
                    .parse()
                    .context("Failed to parse val_bpb")?;
                let status = parts[6].trim().to_string();

                if !agent.is_empty() && agent != "Agent" {
                    rows.push(TableRow {
                        agent,
                        exp_id,
                        config,
                        train_bpb,
                        val_bpb,
                        status,
                    });
                }
            }
        }
    }

    Ok(rows)
}

/// Update table row or append new row
pub fn update_table(body: &str, new_row: &TableRow) -> Result<String> {
    let lines: Vec<&str> = body.lines().collect();
    let mut output = Vec::new();
    let mut table_found = false;
    let mut row_updated = false;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // Find table start
        if line.contains("| Agent | Exp ID") {
            table_found = true;
            output.push(line); // Header
            i += 1;
            output.push(lines[i]); // Separator
            i += 1;

            // Find matching row
            while i < lines.len() {
                let table_line = lines[i];
                if !table_line.starts_with('|') {
                    break;
                }

                let parts: Vec<&str> = table_line.split('|').collect();
                if parts.len() >= 3 && parts[1].trim() == new_row.agent && parts[2].trim() == new_row.exp_id {
                    // Update existing row
                    output.push(&new_row.format_row());
                    row_updated = true;
                } else {
                    output.push(table_line);
                }
                i += 1;
            }

            // If row not found, append it
            if !row_updated {
                output.push(&new_row.format_row());
            }
        } else {
            output.push(line);
        }
        i += 1;
    }

    // If table not found at all, create it
    if !table_found {
        output.push("");
        output.push("## Results");
        output.push("");
        output.push("| Agent | Exp ID | Config | Train BPB | Val BPB | Status |");
        output.push("|-------|--------|--------|-----------|---------|--------|");
        output.push(&new_row.format_row());
    }

    Ok(output.join("\n"))
}

/// Generate summary section for table
pub fn generate_summary(rows: &[TableRow]) -> String {
    let mut summary = String::new();

    writeln!(summary, "## Summary").unwrap();
    writeln!(summary).unwrap();

    // Count by status
    let mut by_status = std::collections::HashMap::new();
    for row in rows {
        *by_status.entry(row.status.clone()).or_insert(0) += 1;
    }

    writeln!(summary, "**By Status:**").unwrap();
    for (status, count) in by_status.iter() {
        writeln!(summary, "- {}: {}", status, count).unwrap();
    }

    writeln!(summary).unwrap();

    // Best val BPB
    if let Some(best) = rows.iter().min_by(|a, b| a.val_bpb.partial_cmp(&b.val_bpb).unwrap()) {
        writeln!(summary, "**Best Val BPB:** {:.4} ({}: {})", best.val_bpb, best.agent, best.exp_id).unwrap();
    }

    summary
}
