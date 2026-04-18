/**
 * @license AGPL-3.0-or-later
 * Copyright 2026 TRIOS
 *
 * TRIOS MCP Bridge Tool — tri_run
 *
 * Executes tri (t27 CLI) command with optional parameters.
 * Returns structured output with all command details.
 */

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
use trios_core::types::{TriRunResult, TriRunCommand};

/// Execute tri CLI and return structured result
pub async fn tri_run(
    working_dir: &str,
    command: Option<TriRunCommand>,
    args: Vec<String>,
) -> Result<Value> {
    // Build tri command with optional arguments
    let mut tri_cmd = Command::new("tri");

    if let Some(working_dir) = working_dir {
        let dir = Path::new(working_dir);
        tri_cmd.arg("--working-dir");
        tri_cmd.arg(&dir);
    }

    match command {
        Some(TriRunCommand::GitStatus) => {
            tri_cmd.args(["status", "--porcelain"]);
        }
        Some(TriRunCommand::GitStageFiles) => {
            tri_cmd.args(["stage"]);
            // Add files from args if provided
            if !args.is_empty() {
                for arg in &args {
                    if arg.starts_with("--") || arg.starts_with("-") {
                        tri_cmd.arg(arg);
                    }
                }
            }
        }
        Some(TriRunCommand::GitCommit) => {
            let message = args
                .first()
                .ok_or_else(|| "no message provided")
                .unwrap();
            tri_cmd.args(["commit", "-m", message]);
        }
        Some(TriRunCommand::GitCreateBranch) => {
            let branch = args
                .first()
                .ok_or_else(|| "no branch name provided")
                .unwrap();
            tri_cmd.args(["branch", branch]);
        }
        Some(TriRunCommand::TriPushStack) => {
            let branch = args
                .first()
                .ok_or_else(|| "no branch name provided")
                .unwrap();
            tri_cmd.args(["push", branch]);
        }
        Some(TriRunCommand::GitPull) => {
            tri_cmd.args(["pull"]);
        }
        Some(TriRunCommand::GitLog) => {
            let lines = args.parse::<usize>().unwrap_or(10);
            tri_cmd.args(["log", "-n", lines.to_string()]);
        }
        Some(TriRunCommand::TriStash) => {
            let message = args
                .first()
                .ok_or_else(|| "no message provided")
                .unwrap();
            tri_cmd.args(["stash", "save", "-m", message]);
        }
        _ => bail!("Unknown command: {command:?}"),
    }

    // Set working directory for tri command
    tri_cmd.current_dir(if let Some(working_dir) = working_dir {
        Path::new(working_dir)
    });

    let output = tri_cmd.output()?;

    let result = TriRunResult {
        command: command.map(|c| c.to_string()).unwrap_or_else("unknown".to_string()),
        args: args.clone(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout: output.stdout.clone(),
        stderr: output.stderr.clone(),
    success: output.status.success(),
    };

    let json_result = serde_json::to_value(result)?;
    Ok(json_result)
}

/// Register tri_run as a tri-server tool
#[trios_core::macros::register_tool]
pub fn register_tool() {
    trios_core::macros::register_tool("tri_run", "Execute tri CLI commands (status, stage, commit, branch, push, pull, log, stash) with optional arguments. Returns structured output including stdout, stderr, exit code, and timing info.")
}
