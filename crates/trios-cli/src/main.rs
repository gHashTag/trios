use anyhow::Result;
use clap::{Parser, Subcommand};

/// tri — IGLA agent CLI (Trinity Parameter Golf)
#[derive(Parser)]
#[command(name = "tri", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run experiment with N seeds
    Run {
        exp_id: String,
        #[arg(long, default_value_t = 1)]
        seeds: u32,
    },
    /// Sweep a parameter over multiple values
    Sweep {
        param: String,
        values: Vec<String>,
    },
    /// Report agent status (optionally with BPB)
    Report {
        agent: String,
        status: String,
        #[arg(long)]
        bpb: Option<f64>,
    },
    /// Atomic commit with message (L8: push first)
    Commit { msg: String },
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Run { exp_id, seeds } => {
            println!("[tri] RUN exp={exp_id} seeds={seeds}");
            todo!("implement: launch igla-trainer with exp config")
        }
        Cmd::Sweep { param, values } => {
            println!("[tri] SWEEP param={param} values={values:?}");
            todo!("implement: iterate values, emit experience/ logs")
        }
        Cmd::Report { agent, status, bpb } => {
            let bpb_str = bpb.map(|b| format!(" bpb={b:.4}")).unwrap_or_default();
            println!("[tri] REPORT agent={agent} status={status}{bpb_str}");
            todo!("implement: write .trinity/experience/AGENT-TIMESTAMP.md")
        }
        Cmd::Commit { msg } => {
            println!("[tri] COMMIT msg={msg:?}");
            todo!("implement: git add -A && git commit -m && git push (L8)")
        }
    }
}
