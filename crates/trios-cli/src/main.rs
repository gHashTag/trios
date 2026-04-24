//! `tri` — Trinity IGLA Needle Hunt CLI
//!
//! Main entry point for tri CLI tool.

use anyhow::Result;
use clap::{Parser, Subcommand};
use trios_cli::{
    cmd::{
        agent::agent_dispatch,
        commit::commit,
        dash::{dash_refresh, dash_sync},
        gates::{gate_check, GateStatus},
        issue::{issue_close, issue_new},
        lang::run as lang_run,
        leaderboard::leaderboard_show,
        race::{init as race_init, status as race_status},
        report::report,
        roster::roster_update,
        railway::{run as railway_run, RailwayCommand},
        run::run,
        status::run as status_run,
        sweep::sweep,
        submit::submit,
        train::train_cpu,
    },
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
        exp_id: String,
        #[arg(long, default_value_t = 1)]
        seeds: u32,
    },

    /// Parameter sweep
    Sweep {
        param: String,
        values: Vec<String>,
    },

    /// Report result to issue #143
    Report {
        agent: String,
        status: String,
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
        agent: String,
        status: String,
    },

    /// Dashboard operations
    Dash {
        #[command(subcommand)]
        sub: DashSub,
    },

    /// Check quality gates
    Gates {
        gate: String,
        #[arg(long)]
        value: Option<f64>,
    },

    /// Submit parameters
    Submit {
        #[arg(long)]
        bpb: f64,
        #[arg(long)]
        artifact: String,
    },

    /// Show leaderboard
    Leaderboard {
        #[arg(long)]
        agent: Option<String>,
    },

    /// Dispatch task to agent
    Agent {
        nato: String,
        task: String,
    },

    /// Git commit (atomic)
    Commit {
        msg: String,
    },

    /// Railway deployment
    Railway {
        #[command(subcommand)]
        sub: RailwayCommand,
    },

    /// Language mode (ru/en)
    Lang {
        lang: String,
    },

    /// Show loop status
    Status {
        #[arg(long)]
        json: bool,
    },

    /// Train CPU n-gram model
    Train {
        #[arg(long, default_value_t = 12000)]
        steps: usize,
        #[arg(long, default_value_t = 128)]
        hidden: usize,
        #[arg(long, default_value_t = 0.004)]
        lr: f64,
        #[arg(long, default_value = "42,43,44")]
        seeds: String,
        #[arg(long, default_value = "relu")]
        activation: String,
        #[arg(long, default_value_t = true)]
        parallel: bool,
        #[arg(long, default_value_t = false)]
        residual: bool,
        #[arg(long, default_value = "0.0")]
        dropout: String,
        #[arg(long, default_value = "0")]
        warmup: String,
        #[arg(long, default_value = "0.04")]
        wd: String,
    },

    /// IGLA RACE commands
    Race {
        #[command(subcommand)]
        sub: RaceSub,
    },
}

#[derive(Subcommand)]
enum IssueSub {
    New { template: String, args: Vec<String> },
    Close { num: u32, #[arg(long)] bpb: Option<f64> },
}

#[derive(Subcommand)]
enum DashSub {
    Sync,
    Refresh,
}

#[derive(Subcommand)]
enum RaceSub {
    /// Initialize IGLA RACE with Optuna study
    Init {
        #[arg(long, default_value = "igla-race")]
        study: String,
        #[arg(long)]
        neon_url: Option<String>,
    },
    /// Show live leaderboard
    Status {
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
}

fn main() -> Result<()> {
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
            report(&agent, &status, bpb)?;
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
            roster_update(&agent, &status)?;
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
                GateStatus::Unknown => println!("❓ Gates unknown"),
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
        Cmd::Railway { sub } => {
            railway_run(sub)?;
        }
        Cmd::Lang { lang } => {
            lang_run(trios_cli::cmd::lang::LangCmd { lang })?;
        }
        Cmd::Status { json } => {
            status_run(trios_cli::cmd::status::StatusCmd { json })?;
        }
        Cmd::Train { steps, hidden, lr, seeds, activation, parallel, residual, dropout, warmup, wd } => {
            let seed_list: Vec<u64> = seeds.split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            let config = trios_cli::cmd::train::TrainConfig {
                steps, hidden, lr, activation, residual, dropout, warmup, wd,
            };
            let results = train_cpu(seed_list, config, parallel)?;
            let avg = results.iter().map(|r| r.best_bpb).sum::<f64>() / results.len() as f64;
            println!("\n📊 Average BPB: {:.3} ({})", avg, results.len());
        }
        Cmd::Race { sub } => match sub {
            RaceSub::Init { study, neon_url } => {
                let config = trios_cli::cmd::race::RaceConfig {
                    study_name: study,
                    neon_url: neon_url.unwrap_or_else(|| std::env::var("NEON_DATABASE_URL").unwrap_or_else(|_| "postgresql://user:pass@ep-xxx.us-east-2.aws.neon.tech/neondb".to_string())),
                    ..Default::default()
                };
                race_init(config)?;
            }
            RaceSub::Status { limit } => {
                race_status(Some(limit))?;
            }
        },
    }
    Ok(())
}
