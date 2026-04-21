use anyhow::Result;
use clap::{Parser, Subcommand};

mod gh;
mod table;

/// tri — IGLA agent CLI (Trinity Parameter Golf)
/// Architecture: each step = tri CLI + experience/ source of truth .trinity
#[derive(Parser)]
#[command(name = "tri", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run experiment with N seeds, push result to experience/
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
    /// Report agent status with optional BPB — writes .trinity/experience/ log
    Report {
        agent: String,
        status: String,
        #[arg(long)]
        bpb: Option<f64>,
    },
    /// Atomic commit: git add -A + commit + push (L8: no push = does not exist)
    Commit { msg: String },
    /// Show RINGS dashboard (issues #143 summary)
    Dash,
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Run { exp_id, seeds } => {
            println!("[tri] RUN exp={exp_id} seeds={seeds}");
            todo!("launch igla-trainer with exp config, write experience/ log")
        }
        Cmd::Sweep { param, values } => {
            println!("[tri] SWEEP param={param} values={values:?}");
            todo!("iterate values, emit .trinity/experience/SWEEP-TIMESTAMP.md")
        }
        Cmd::Report { agent, status, bpb } => {
            let bpb_str = bpb.map(|b| format!(" bpb={b:.4}")).unwrap_or_default();
            println!("[tri] REPORT agent={agent} status={status}{bpb_str}");
            todo!("write .trinity/experience/AGENT-TIMESTAMP.md with AIP footer")
        }
        Cmd::Commit { msg } => {
            println!("[tri] COMMIT msg={msg:?}");
            todo!("git add -A && git commit -m msg && git push (L8)")
        }
        Cmd::Dash => {
            println!("[tri] DASH — fetching #143 roster...");
            todo!("gh issue view 143 --repo gHashTag/trios + render table::Table")
        }
    }
}
