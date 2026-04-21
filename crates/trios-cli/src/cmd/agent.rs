//! `tri agent` — Dispatch task to agent
//!
//! Usage:
//!   tri agent ALFA "run phase_b_fine"
//!   tri agent FOXTROT "sweep lr 0.01 0.0162"

use anyhow::{Context, Result};

use crate::config::Config;

/// Dispatch task to agent
pub fn agent_dispatch(nato: &str, task: &str) -> Result<()> {
    println!("🤖 Dispatching to {}: {}", nato, task);

    let _config = Config::load();

    // Verify agent exists in roster
    let valid_nato = ["ALFA", "BRAVO", "CHARLIE", "DELTA", "ECHO"];
    if !valid_nato.contains(&nato) {
        anyhow::bail!("Unknown NATO code: {}. Use: {:?}", nato, valid_nato);
    }

    // Create task queue file
    let task_dir = ".trinity/tasks";
    std::fs::create_dir_all(task_dir)
        .context("Failed to create task directory")?;

    let task_file = format!("{}/{}.txt", task_dir, nato.to_lowercase());
    let task_line = format!("{}\n", task);

    std::fs::write(&task_file, task_line)
        .context("Failed to write task file")?;

    println!("✓ Task queued for {}", nato);

    Ok(())
}

/// List pending tasks for an agent
pub fn agent_list(nato: Option<&str>) -> Result<()> {
    let task_dir = ".trinity/tasks";

    let agents = if let Some(n) = nato {
        vec![n.to_string()]
    } else {
        vec!["alfa".to_string(), "bravo".to_string(), "charlie".to_string(), "delta".to_string(), "echo".to_string()]
    };

    for agent in agents {
        let task_file = format!("{}/{}.txt", task_dir, agent);

        if let Ok(tasks) = std::fs::read_to_string(&task_file) {
            if !tasks.trim().is_empty() {
                println!("{}:", agent.to_uppercase());
                for line in tasks.lines() {
                    println!("  - {}", line);
                }
            }
        }
    }

    Ok(())
}
