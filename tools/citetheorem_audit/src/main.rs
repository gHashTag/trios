//! `citetheorem-audit` CLI — R14 retrofit witness for the «Flos Aureus»
//! monograph.
//!
//! Audits LaTeX chapter files at `--chapters-dir` for `\citetheorem{INV-N}`
//! coverage against `--assertions <igla_assertions.json>`.  Strict mode
//! (default) exits 0 iff every chapter passes the floor; report mode
//! (`--report`) prints a JSON audit and always exits 0 on healthy I/O.
//!
//! Exit code namespace 80..=83 — disjoint from L7 (0..=10), L15 (21..=30),
//! L-h4 (50..=53), LT (60..=63), and LA (70..=74).
//!
//! See `tools/citetheorem_audit/src/lib.rs` for the binding contract.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use citetheorem_audit::{
    audit_report, audit_strict, render_report_json, CiteAuditError, MIN_CITETHEOREM_PER_CHAPTER,
};

#[derive(Debug, Parser)]
#[command(
    name = "citetheorem-audit",
    about = "R14 retrofit witness — audit \\citetheorem{INV-N} coverage in docs/phd/chapters/",
    long_about = "Validates that every *.tex file under --chapters-dir contains \
                  at least MIN_CITETHEOREM_PER_CHAPTER `\\citetheorem{INV-N}` calls \
                  resolving to an INV id present in --assertions. Strict mode \
                  (default) exits 0 on admit, 80..=83 on reject (one code per \
                  CiteAuditError variant). Report mode (--report) prints a JSON \
                  audit and exits 0 on healthy I/O regardless of R14 verdict. \
                  Pre-reg: ONE SHOT v2.0 §3 Phase D · §5."
)]
struct Args {
    /// Directory containing the chapter `*.tex` files.
    #[arg(long, default_value = "docs/phd/chapters")]
    chapters_dir: PathBuf,

    /// Path to `assertions/igla_assertions.json`.
    #[arg(long, default_value = "assertions/igla_assertions.json")]
    assertions: PathBuf,

    /// Print L-R14 anchors as JSON instead of running the gate.
    #[arg(long, default_value_t = false)]
    print_anchors: bool,

    /// Audit-only mode: print the JSON report and exit 0 unless I/O fails.
    #[arg(long, default_value_t = false)]
    report: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.print_anchors {
        let anchors = serde_json::json!({
            "lane": "citetheorem_audit",
            "phase": "D",
            "rule": "R14",
            "min_citetheorem_per_chapter": MIN_CITETHEOREM_PER_CHAPTER,
            "exit_codes": {
                "admit": 0,
                "assertions_load_error": 80,
                "chapters_dir_error": 81,
                "unknown_inv_cited": 82,
                "below_min_citations": 83,
            },
            "one_shot": "https://github.com/gHashTag/trios/issues/265#issuecomment-4321142675",
            "witness_manifest": "assertions/witness/citetheorem_audit.toml",
            "trinity_anchor": "phi^2 + phi^-2 = 3",
            "zenodo_doi": "10.5281/zenodo.19227877",
        });
        println!("{anchors:#}");
        return ExitCode::SUCCESS;
    }

    if args.report {
        return match audit_report(&args.chapters_dir, &args.assertions) {
            Ok(report) => match render_report_json(&report) {
                Ok(json) => {
                    println!("{json}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("REJECT: render_report_json: {e}");
                    ExitCode::from(80) // I/O class — surface as load-error namespace.
                }
            },
            Err(e) => fail(e),
        };
    }

    match audit_strict(&args.chapters_dir, &args.assertions) {
        Ok(report) => {
            println!(
                "ADMIT: {n} chapter(s), all >= {min} \\citetheorem call(s).",
                n = report.rows.len(),
                min = MIN_CITETHEOREM_PER_CHAPTER,
            );
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

fn fail(e: CiteAuditError) -> ExitCode {
    let code = e.exit_code();
    eprintln!("REJECT: {e}");
    ExitCode::from(code)
}
