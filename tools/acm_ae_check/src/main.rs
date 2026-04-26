//! `acm-ae-check` CLI — LA Phase D witness for the «Flos Aureus» monograph.
//!
//! Subcommands:
//!
//! - `run` — execute Functional + Reusable + Available checks, write
//!   `artifact/output.txt`, diff against `artifact/expected.txt`.
//!   Exits 0 on full admit, 70..=74 on reject.
//! - `print` — print the deterministic fingerprint to stdout (used to
//!   regenerate `artifact/expected.txt` when the LA contract is
//!   re-pre-registered).
//!
//! See `tools/acm_ae_check/src/lib.rs` for the binding contract and L-R14
//! traceability table.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use acm_ae_check::{exit, fingerprint, run_all, AcmAeError};

#[derive(Debug, Parser)]
#[command(
    name = "acm-ae-check",
    about = "LA ACM AE 3-badge witness for trios#265 Phase D",
    long_about = "Validates that the working tree carries Functional + Reusable + Available \
                  evidence for the «Flos Aureus» PhD monograph and that \
                  artifact/output.txt matches artifact/expected.txt byte-for-byte. \
                  Exit codes 70..=74 (disjoint from L-h4 50..=53 and LT 60..=63)."
)]
struct Args {
    /// Path to the trios checkout (default: cwd).
    #[arg(long, default_value = ".")]
    repo: PathBuf,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Run the full 3-badge witness; write output.txt and diff against expected.txt.
    Run {
        /// Skip writing `artifact/output.txt` (audit-only).
        #[arg(long, default_value_t = false)]
        no_write: bool,
    },
    /// Print the deterministic fingerprint to stdout.
    Print,
    /// Print the L-R14 anchor map as JSON.
    Anchors,
}

fn main() -> ExitCode {
    let args = Args::parse();

    match args.cmd {
        Cmd::Print => {
            print!("{}", fingerprint());
            ExitCode::SUCCESS
        }
        Cmd::Anchors => {
            let j = serde_json::json!({
                "lane": "LA",
                "phase": "D",
                "anchor": acm_ae_check::TRINITY_ANCHOR,
                "doi": acm_ae_check::ZENODO_DOI,
                "min_pages": acm_ae_check::MIN_PAGES,
                "max_pages": acm_ae_check::MAX_PAGES,
                "prune_threshold": acm_ae_check::PRUNE_THRESHOLD,
                "warmup_blind_steps": acm_ae_check::WARMUP_BLIND_STEPS,
                "d_model_min": acm_ae_check::D_MODEL_MIN,
                "lr_champion": acm_ae_check::LR_CHAMPION,
                "exit_codes": {
                    "admit": exit::ADMIT,
                    "functional": exit::FUNCTIONAL,
                    "reusable": exit::REUSABLE,
                    "available": exit::AVAILABLE,
                    "io": exit::IO,
                    "mismatch": exit::MISMATCH,
                },
                "one_shot": "https://github.com/gHashTag/trios/issues/265#issuecomment-4321142675",
                "witness_manifest": "assertions/witness/acm_ae.toml",
            });
            println!("{j:#}");
            ExitCode::SUCCESS
        }
        Cmd::Run { no_write } => {
            let repo = args.repo;
            let out_path = repo.join("artifact/output.txt");
            if !no_write {
                if let Err(e) = std::fs::create_dir_all(repo.join("artifact")) {
                    eprintln!("REJECT: io error creating artifact/: {e}");
                    return ExitCode::from(exit::IO);
                }
                if let Err(e) = std::fs::write(&out_path, fingerprint()) {
                    eprintln!("REJECT: io error writing {}: {e}", out_path.display());
                    return ExitCode::from(exit::IO);
                }
            }
            match run_all(&repo) {
                Ok(report) => {
                    println!(
                        "ADMIT: ACM AE 3-badge PASS (Functional, Reusable, Available); \
                         lane={} phase={} anchor=\"{}\"",
                        report.lane, report.phase, report.anchor
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    let code = match &e {
                        AcmAeError::Functional { .. } => exit::FUNCTIONAL,
                        AcmAeError::Reusable { .. } => exit::REUSABLE,
                        AcmAeError::Available { .. } => exit::AVAILABLE,
                        AcmAeError::Io { .. } => exit::IO,
                        AcmAeError::Mismatch { .. } => exit::MISMATCH,
                    };
                    eprintln!("REJECT: {e}");
                    ExitCode::from(code)
                }
            }
        }
    }
}
