//! `trios-phd` CLI — `cargo run -p trios-phd -- audit`.
//!
//! Subcommands:
//!
//! - `audit` — run [`trios_phd::audit::run_audit`] against the repo root and
//!   exit non-zero if any honesty rule (R4 / R5) is violated.
//!
//! Future subcommands tracked in `docs/phd/BRIDGE_AUDIT.md`:
//! `compile`, `generate-figure`, `export-trials`, `bibtex-check`.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "trios-phd",
    version,
    about = "Flos Aureus PhD pipeline (skeleton — closes #62 partially)"
)]
struct Cli {
    /// Repository root. Defaults to current directory.
    #[arg(long, default_value = ".")]
    repo: PathBuf,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run the L-R14 / R5 honesty audit across `assertions/` and `docs/phd/`.
    Audit,
    /// Print the JSON-derived invariant status table.
    Status,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Audit => {
            let report = trios_phd::audit::run_audit(&cli.repo)?;
            println!("{}", report.render());
            if report.is_clean() {
                println!("audit: OK");
                Ok(())
            } else {
                std::process::exit(2);
            }
        }
        Cmd::Status => {
            let a = trios_phd::audit::Assertions::load(
                cli.repo.join("assertions/igla_assertions.json"),
            )?;
            for inv in &a.invariants {
                let status = match inv.status {
                    trios_phd::audit::ProofStatus::Proven => "Proven",
                    trios_phd::audit::ProofStatus::Admitted => "Admitted",
                };
                println!("{}: {} ({})", inv.id, status, inv.coq_file);
            }
            Ok(())
        }
    }
}
