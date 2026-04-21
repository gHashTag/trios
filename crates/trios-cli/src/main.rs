//! `tri` — Trinity IGLA Needle Hunt CLI
//!
//! Main entry point for the tri CLI tool.

use anyhow::Result;
use clap::{Parser, Subcommand};

use tri::{
    cmd::{
        agent::{agent_dispatch, agent_list},
        commit::{commit, commit_add},
        dash::{dash_refresh, dash_sync},
        gates::{gate_check, GateStatus},
        issue::{issue_close, issue_new},
        leaderboard::{leaderboard_export, leaderboard_show},
        report::{report, report_batch},
        roster::{roster_show, roster_update},
        run::{run, RunResult},
        sweep::{sweep, SweepResults},
        submit::submit,
    },
    Config,
};

#[derive(Parser)]
#[command(name = "tri", version = "0.1.0", about = "Trinity IGLA CLI — Needle Hunt Automation")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run experiment and parse BPB
    Run {
        /// Experiment ID (e.g., phase_b_fine)
        exp_id: String,

        /// Number of seeds to run (default: 1)
        #[arg(long, default_value_t = 1)]
        seeds: u32,
    },

    /// Parameter sweep
    Sweep {
        /// Parameter to sweep (e.g., lr, hidden)
        param: String,

        /// Values to test
        values: Vec<String>,
    },

    /// Report result to issue #143
    Report {
        /// Agent NATO code (e.g., ALFA, BRAVO)
        agent: String,

        /// Status (e.g., complete, running, failed)
        status: String,

        /// BPB value (optional)
        #[arg(long)]
        bpb: Option<f64>,
    },

    /// Issue management
    Issue {
        #[command(subcommand)]
        sub: IssueSub,
    },

    /// Update agent roster
    Roster {
        /// Agent NATO code
        agent: String,

        /// New status
        status: String,
    },

    /// Dashboard operations
    Dash {
        #[command(subcommand)]
        sub: DashSub,
    },

    /// Check quality gates
    Gates {
        /// Gate to check (bpab, size, time, all)
        gate: String,

        /// Value for gate check
        #[arg(long)]
        value: Option<f64>,
    },

    /// Submit parameters
    Submit {
        /// BPB result
        #[arg(long)]
        bpb: f64,

        /// Artifact path
        #[arg(long)]
        artifact: String,
    },

    /// Show leaderboard
    Leaderboard {
        /// Filter by agent
        #[arg(long)]
        agent: Option<String>,
    },

    /// Dispatch task to agent
    Agent {
        /// Agent NATO code
        nato: String,

        /// Task description
        task: String,
    },

    /// Git commit (atomic)
    Commit {
        /// Commit message
        msg: String,
    },
}

#[derive(Subcommand)]
enum IssueSub {
    /// Create new issue
    New {
        /// Template (experiment, bug, feature)
        template: String,

        /// Template arguments
        args: Vec<String>,
    },

    /// Close issue
    Close {
        /// Issue number
        num: u32,

        /// Final BPB (optional)
        #[arg(long)]
        bpb: Option<f64>,
    },
}

#[derive(Subcommand)]
enum DashSub {
    /// Sync dashboard with GitHub
    Sync,

    /// Refresh dashboard metrics
    Refresh,
}

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    match Cli::parse().cmd {
        Cmd::Run { exp_id, seeds } => {
            run(&exp_id, seeds)?;
        }

        Cmd::Sweep { param, values } => {
            let results = sweep(&param, values)?;
            println!();
            println!("{}", results.to_markdown(&param));
        }

        Cmd::Report { agent, status, bpb } => {
            report(agent, status, bpb)?;
        }

        Cmd::Issue { sub } => match sub {
            IssueSub::New { template, args } => {
                issue_new(&template, &args)?;
            }
            IssueSub::Close { num, bpb } => {
                issue_close(num, bpb)?;
            }
        },

        Cmd::Roster { agent, status } => {
            roster_update(agent, status)?;
        }

        Cmd::Dash { sub } => match sub {
            DashSub::Sync => dash_sync()?,
            DashSub::Refresh => dash_refresh()?,
        },

        Cmd::Gates { gate, value } => {
            let status = gate_check(&gate, value)?;
            println!();
            match status {
                GateStatus::Pass => println!("✅ All gates passed"),
                GateStatus::Warn => println!("⚠️  Some gates warn"),
                GateStatus::Fail => println!("❌ Gates failed"),
                GateStatus::Unknown => println!("❓ Gates unknown (no values provided)"),
            }
        }

        Cmd::Submit { bpb, artifact } => {
            submit(bpb, &artifact)?;
        }

        Cmd::Leaderboard { agent } => {
            leaderboard_show(agent.as_deref())?;
        }

        Cmd::Agent { nato, task } => {
            agent_dispatch(&nato, &task)?;
        }

        Cmd::Commit { msg } => {
            commit(&msg)?;
        }
    }

    Ok(())
}
