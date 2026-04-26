//! `defense-gate` CLI — Phase C LD defense-package witness.
//!
//! Inspects `docs/phd/defense/{examiner-pack,qa,slides}.tex` and
//! verifies that each meets its pre-registered floor:
//!
//! - examiner-pack ≥ 50 sectioned units (`\section` / `\subsection` /
//!   `\subsubsection`)
//! - qa            ≥ 30 Q/A pairs
//! - slides        ≥ 30 Beamer frames
//!
//! Strict mode (default) exits 0 on admit, 100..=104 on reject (one
//! code per [`DefenseGateError`] variant).  Report mode (`--report`)
//! prints a JSON audit and exits 0 on healthy I/O regardless of
//! verdict.
//!
//! Exit code namespace 100..=104 — disjoint from L7 (0..=10), L15
//! (21..=30), L-h4 (50..=53), LT (60..=63), LA (70..=74),
//! citetheorem_audit (80..=83), and merge_order_gate (90..=93).
//!
//! See `tools/defense_gate/src/lib.rs` for the binding contract.

use std::process::ExitCode;

use clap::Parser;

use defense_gate::{
    audit_report, audit_strict, render_report_json, resolve_defense_dir, DefenseAnchors,
    DefenseGateError, EXAMINER_MIN_SECTIONS, QA_MIN_PAIRS, SLIDES_MIN_FRAMES,
};

#[derive(Debug, Parser)]
#[command(
    name = "defense-gate",
    about = "Phase C LD defense-package gate — examiner ≥50 §, qa ≥30, slides ≥30",
    long_about = "Audits docs/phd/defense/{examiner-pack,qa,slides}.tex against the \
                  pre-registered floors EXAMINER_MIN_SECTIONS=50, QA_MIN_PAIRS=30, \
                  SLIDES_MIN_FRAMES=30. Strict mode (default) exits 0 on admit, \
                  100..=104 on reject. Report mode (--report) prints a JSON audit and \
                  exits 0 on healthy I/O. Pre-reg: ONE SHOT v2.0 §3 Phase C · §5 row 4."
)]
struct Args {
    /// Path to the defense directory (default `docs/phd/defense`).
    #[arg(long)]
    defense_dir: Option<String>,

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
        let anchors = DefenseAnchors::from_registry();
        let payload = serde_json::json!({
            "lane": "defense_gate",
            "phase": "C",
            "rule": "Phase C LD defense-package floors",
            "anchors": anchors,
            "exit_codes": {
                "admit": 0,
                "defense_dir_error": 100,
                "examiner_pack_too_short": 101,
                "qa_pairs_below_floor": 102,
                "slides_below_floor": 103,
                "missing_defense_file": 104,
            },
            "floors_const_view": {
                "examiner_min_sections": EXAMINER_MIN_SECTIONS,
                "qa_min_pairs": QA_MIN_PAIRS,
                "slides_min_frames": SLIDES_MIN_FRAMES,
            },
            "one_shot": "https://github.com/gHashTag/trios/issues/265#issuecomment-4321142675",
            "witness_manifest": "assertions/witness/defense_gate.toml",
            "trinity_anchor": "phi^2 + phi^-2 = 3",
            "zenodo_doi": "10.5281/zenodo.19227877",
        });
        println!("{payload:#}");
        return ExitCode::SUCCESS;
    }

    let defense_dir = resolve_defense_dir(args.defense_dir.as_deref());

    if args.report {
        return match audit_report(&defense_dir) {
            Ok(report) => {
                println!("{}", render_report_json(&report));
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        };
    }

    match audit_strict(&defense_dir) {
        Ok(()) => {
            println!(
                "ADMIT: defense package at {} meets floors ({}/{}/{}).",
                defense_dir.display(),
                EXAMINER_MIN_SECTIONS,
                QA_MIN_PAIRS,
                SLIDES_MIN_FRAMES
            );
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

fn fail(e: DefenseGateError) -> ExitCode {
    let code = e.exit_code();
    eprintln!("REJECT: {e}");
    ExitCode::from(code as u8)
}
