//! `page-gate` CLI — LT Phase D witness for the «Flos Aureus» monograph.
//!
//! Verifies that the PDF at `--pdf <path>` has a page count inside the
//! pre-registered band `[MIN_PAGES, MAX_PAGES]`.  Exits 0 on admit; 60..=63
//! on reject (one code per [`PageGateError`] variant).
//!
//! See `tools/page_gate/src/lib.rs` for the binding contract.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use page_gate::{check_pdf, MAX_PAGES, MIN_PAGES};

#[derive(Debug, Parser)]
#[command(
    name = "page-gate",
    about = "LT page-count gate for trios#265 Phase D",
    long_about = "Validates that a tectonic-built PDF has a page count inside \
                  the pre-registered band [MIN_PAGES, MAX_PAGES]. Exits 0 on \
                  admit; 60..=63 on reject. Pre-reg: ONE SHOT v2.0 §3 Phase D."
)]
struct Args {
    /// Path to the PDF to gate (default: docs/phd/main.pdf).
    #[arg(long, default_value = "docs/phd/main.pdf")]
    pdf: PathBuf,

    /// Print L-R14 anchors as JSON instead of running the gate.
    #[arg(long, default_value_t = false)]
    print_anchors: bool,

    /// Print the page count without invoking the gate (audit mode).
    #[arg(long, default_value_t = false)]
    count_only: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.print_anchors {
        let anchors = serde_json::json!({
            "lane": "LT",
            "phase": "D",
            "min_pages": MIN_PAGES,
            "max_pages": MAX_PAGES,
            "exit_codes": {
                "admit": 0,
                "below_band": 60,
                "above_band": 61,
                "io": 62,
                "malformed": 63,
            },
            "one_shot": "https://github.com/gHashTag/trios/issues/265#issuecomment-4321142675",
            "witness_manifest": "assertions/witness/page_gate.toml",
            "trinity_anchor": "phi^2 + phi^-2 = 3",
            "zenodo_doi": "10.5281/zenodo.19227877",
        });
        println!("{anchors:#}");
        return ExitCode::SUCCESS;
    }

    if args.count_only {
        match page_gate::count_pages(&args.pdf) {
            Ok(n) => {
                println!("{n}");
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("REJECT: {e}");
                return ExitCode::from(e.exit_code());
            }
        }
    }

    match check_pdf(&args.pdf) {
        Ok(n) => {
            println!(
                "ADMIT: {pdf} has {n} pages (band [{lo}, {hi}])",
                pdf = args.pdf.display(),
                lo = MIN_PAGES,
                hi = MAX_PAGES,
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("REJECT: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}
