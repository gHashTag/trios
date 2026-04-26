//! `merge-order-gate` CLI — Phase A merge-order witness.
//!
//! Reads a `git log --merges --first-parent --pretty='%h %s'` snapshot
//! at `--git-log <path>` and verifies that for every chapter merge
//! (`feat/phd-ch*`), all pre-registered Phase A prerequisites
//! (`feat/phd-appendix-F-restore`, `feat/phd-lf-frontmatter`,
//! `feat/phd-lb-springer`) appear at strictly earlier indices.
//!
//! Strict mode (default) exits 0 on admit, 90..=93 on reject (one code
//! per [`MergeOrderError`] variant).  Report mode (`--report`) prints
//! a JSON audit and exits 0 on healthy I/O regardless of verdict.
//!
//! Exit code namespace 90..=93 — disjoint from L7 (0..=10), L15
//! (21..=30), L-h4 (50..=53), LT (60..=63), LA (70..=74), and
//! citetheorem_audit (80..=83).
//!
//! See `tools/merge_order_gate/src/lib.rs` for the binding contract.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use merge_order_gate::{
    audit_report, audit_strict, render_report_json, MergeOrderError, CHAPTER_PREFIX,
    PHASE_A_PREREQS,
};

#[derive(Debug, Parser)]
#[command(
    name = "merge-order-gate",
    about = "Phase A merge-order gate — refuses chapter merges before #288/#289/#294",
    long_about = "Validates that for every regulated chapter merge in --git-log, \
                  all pre-registered Phase A prerequisite branches \
                  (feat/phd-appendix-F-restore, feat/phd-lf-frontmatter, \
                  feat/phd-lb-springer) appear at strictly earlier indices in the \
                  first-parent transcript. Strict mode (default) exits 0 on admit, \
                  90..=93 on reject. Report mode (--report) prints a JSON audit and \
                  exits 0 on healthy I/O. Pre-reg: ONE SHOT v2.0 §3 Phase A · §5."
)]
struct Args {
    /// Path to a `git log --merges --first-parent --pretty='%h %s'` snapshot.
    #[arg(long, default_value = "git.log")]
    git_log: PathBuf,

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
            "lane": "merge_order_gate",
            "phase": "A",
            "rule": "Phase A topological merge order",
            "phase_a_prereqs": PHASE_A_PREREQS,
            "chapter_prefix": CHAPTER_PREFIX,
            "exit_codes": {
                "admit": 0,
                "git_log_read_error": 90,
                "malformed_merge_entry": 91,
                "chapter_before_prereq": 92,
                "prereq_never_merged": 93,
            },
            "one_shot": "https://github.com/gHashTag/trios/issues/265#issuecomment-4321142675",
            "witness_manifest": "assertions/witness/merge_order_gate.toml",
            "trinity_anchor": "phi^2 + phi^-2 = 3",
            "zenodo_doi": "10.5281/zenodo.19227877",
        });
        println!("{anchors:#}");
        return ExitCode::SUCCESS;
    }

    if args.report {
        return match audit_report(&args.git_log) {
            Ok(report) => match render_report_json(&report) {
                Ok(json) => {
                    println!("{json}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("REJECT: render_report_json: {e}");
                    ExitCode::from(90) // I/O class
                }
            },
            Err(e) => fail(e),
        };
    }

    match audit_strict(&args.git_log) {
        Ok(report) => {
            println!(
                "ADMIT: {n_rows} merge(s); {n_chapters} chapter merge(s); \
                 {n_prereqs} of {n_required} Phase A prereq(s) seen.",
                n_rows = report.rows.len(),
                n_chapters = report.chapter_merges.len(),
                n_prereqs = report.prereqs_seen.len(),
                n_required = PHASE_A_PREREQS.len(),
            );
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

fn fail(e: MergeOrderError) -> ExitCode {
    let code = e.exit_code();
    eprintln!("REJECT: {e}");
    ExitCode::from(code)
}
