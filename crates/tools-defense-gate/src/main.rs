//! LD lane falsification witness for the Flos Aureus defense package.
//!
//! Per ONE SHOT v2.0 (trios#265 comment 4321142675), the LD lane carries a
//! single witness binary. It exits with a non-zero status iff any of the
//! following gates fail:
//!
//!   * `docs/phd/defense/examiner-pack.tex` exists with `>= 600` LaTeX lines
//!     (target typeset is `<= 50` pages; `>= 600` lines is the conservative
//!     proxy used by the auditor cycle).
//!   * `docs/phd/defense/anticipated-questions.tex` carries exactly 30
//!     `\paragraph{Q1}..{Q30}` blocks.
//!   * `docs/phd/defense/slides/main.tex` carries exactly 30
//!     `\begin{frame}` blocks.
//!   * `docs/phd/defense/public-summary.md` exists.
//!   * `docs/phd/defense/rehearsal-log.md` exists.
//!
//! R-rule trace:
//!   * R1 — Rust only (no `.py`, no `.sh` business logic).
//!   * R5 — exit codes are honest: zero iff *all* gates pass.
//!   * R7 — every gate is a witness; failure produces a falsifier message.
//!   * R10 — atomic per-PR; one binary, one acceptance test.
//!
//! Run:
//!   cargo run -p tools-defense-gate
//!
//! Auditor: phd-monograph-auditor v1.0; lane LD; agent perplexity-computer-ld-defense.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

/// Minimum LaTeX line count for the examiner pack (proxy for `>= 50` typeset pages).
const EXAMINER_PACK_MIN_LINES: usize = 600;

/// Required Q&A pair count.
const QA_REQUIRED_COUNT: usize = 30;

/// Required Beamer frame count.
const SLIDES_REQUIRED_COUNT: usize = 30;

/// Defense-package paths (relative to the workspace root).
const EXAMINER_PACK: &str = "docs/phd/defense/examiner-pack.tex";
const QA_FILE: &str = "docs/phd/defense/anticipated-questions.tex";
const SLIDES_FILE: &str = "docs/phd/defense/slides/main.tex";
const PUBLIC_SUMMARY: &str = "docs/phd/defense/public-summary.md";
const REHEARSAL_LOG: &str = "docs/phd/defense/rehearsal-log.md";

#[derive(Debug)]
enum GateResult {
    Pass(String),
    Fail(String),
}

fn check_file_exists(path: &str) -> GateResult {
    if Path::new(path).is_file() {
        GateResult::Pass(format!("OK   {} exists", path))
    } else {
        GateResult::Fail(format!("FAIL {} missing", path))
    }
}

fn check_examiner_pack() -> GateResult {
    let content = match fs::read_to_string(EXAMINER_PACK) {
        Ok(s) => s,
        Err(e) => return GateResult::Fail(format!("FAIL {} unreadable: {}", EXAMINER_PACK, e)),
    };
    let lines = content.lines().count();
    if lines >= EXAMINER_PACK_MIN_LINES {
        GateResult::Pass(format!(
            "OK   {} has {} lines (>= {})",
            EXAMINER_PACK, lines, EXAMINER_PACK_MIN_LINES
        ))
    } else {
        GateResult::Fail(format!(
            "FAIL {} has {} lines (< {})",
            EXAMINER_PACK, lines, EXAMINER_PACK_MIN_LINES
        ))
    }
}

fn check_qa_count() -> GateResult {
    let content = match fs::read_to_string(QA_FILE) {
        Ok(s) => s,
        Err(e) => return GateResult::Fail(format!("FAIL {} unreadable: {}", QA_FILE, e)),
    };
    // Each Q&A block starts with `\paragraph{Q<n>` (e.g. Q1, Q2, ..., Q30).
    let count = content
        .lines()
        .filter(|line| line.trim_start().starts_with("\\paragraph{Q"))
        .count();
    if count == QA_REQUIRED_COUNT {
        GateResult::Pass(format!(
            "OK   {} has {} Q&A pairs (== {})",
            QA_FILE, count, QA_REQUIRED_COUNT
        ))
    } else {
        GateResult::Fail(format!(
            "FAIL {} has {} Q&A pairs (!= {})",
            QA_FILE, count, QA_REQUIRED_COUNT
        ))
    }
}

fn check_slides_count() -> GateResult {
    let content = match fs::read_to_string(SLIDES_FILE) {
        Ok(s) => s,
        Err(e) => return GateResult::Fail(format!("FAIL {} unreadable: {}", SLIDES_FILE, e)),
    };
    // Beamer frames: count `\begin{frame}` occurrences.
    let count = content
        .lines()
        .filter(|line| line.trim_start().starts_with("\\begin{frame}"))
        .count();
    if count == SLIDES_REQUIRED_COUNT {
        GateResult::Pass(format!(
            "OK   {} has {} frames (== {})",
            SLIDES_FILE, count, SLIDES_REQUIRED_COUNT
        ))
    } else {
        GateResult::Fail(format!(
            "FAIL {} has {} frames (!= {})",
            SLIDES_FILE, count, SLIDES_REQUIRED_COUNT
        ))
    }
}

fn run_gates() -> Vec<GateResult> {
    vec![
        check_examiner_pack(),
        check_qa_count(),
        check_slides_count(),
        check_file_exists(PUBLIC_SUMMARY),
        check_file_exists(REHEARSAL_LOG),
    ]
}

fn main() -> ExitCode {
    println!("LD defense-gate witness — phd-monograph-auditor v1.0");
    println!("=====================================================");
    let results = run_gates();
    let mut failed = 0;
    for r in &results {
        match r {
            GateResult::Pass(msg) => println!("{}", msg),
            GateResult::Fail(msg) => {
                println!("{}", msg);
                failed += 1;
            }
        }
    }
    println!("=====================================================");
    if failed == 0 {
        println!("All {} gates pass. φ² + φ⁻² = 3.", results.len());
        ExitCode::SUCCESS
    } else {
        println!("{} of {} gates fail. Defense package incomplete.", failed, results.len());
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_one_shot_v2() {
        assert_eq!(QA_REQUIRED_COUNT, 30, "ONE SHOT v2.0 fixes Q&A count at 30");
        assert_eq!(SLIDES_REQUIRED_COUNT, 30, "ONE SHOT v2.0 fixes slide count at 30");
        assert!(
            EXAMINER_PACK_MIN_LINES >= 600,
            "ONE SHOT v2.0 examiner pack must be >= 50 typeset pages (>= 600 lines proxy)"
        );
    }
}
