//! `defense_gate` — Phase C LD defense-package witness library.
//!
//! Pure structural witness for the «Flos Aureus» PhD monograph
//! (trios#265 ONE SHOT v2.0 §5 row 4):
//!
//! > *examiner-pack <50 pp, Q&A <30, slides <30*
//!
//! ## What this enforces
//!
//! Phase C of the ONE SHOT prescribes a three-file LD defense package
//! under `docs/phd/defense/`:
//!
//! 1. `examiner-pack.tex` — 50 pp examiner pack (cover + ToC +
//!    10 × §answer + appendix).  Counted as **≥
//!    [`EXAMINER_MIN_SECTIONS`] sectioned units** (`\section` /
//!    `\subsection` / `\subsubsection`) — a structural proxy for "50 pp"
//!    that does not require shelling out to `pdfinfo` (R1: Rust-only).
//! 2. `qa.tex` — **≥ [`QA_MIN_PAIRS`] Q/A blocks** counted by any of
//!    `\question`, `\qaitem`, or `\textbf{Q` markers.
//! 3. `slides.tex` — Beamer source with **≥ [`SLIDES_MIN_FRAMES`]
//!    `\begin{frame}` blocks** (or self-closing `\frame{...}`).
//!
//! On `main` at the time this crate landed, **`docs/phd/defense/` does
//! not exist** — LD body fill is queue-bot-blocked behind PRs #304/#305.
//! The gate is therefore vacuously REJECT(104) (`MissingDefenseFile`)
//! today and becomes load-bearing the moment LD lands on `main`.
//!
//! ## R1 / R6 / R10 compliance
//!
//! - R1: pure Rust.  Replaces the `.sh` placeholder named in §5 row 4.
//! - R6: greenfield `tools/defense_gate/`.
//! - R10: ships in one atomic commit per the ONE SHOT contract.
//!
//! ## R8 falsification witnesses
//!
//! Each rejection variant of [`DefenseGateError`] is paired with a
//! `falsify_*` unit test that constructs the failing fixture and
//! asserts the corresponding exit code.
//!
//! ## L-R14 traceability
//!
//! The three floor constants (`EXAMINER_MIN_SECTIONS`, `QA_MIN_PAIRS`,
//! `SLIDES_MIN_FRAMES`) are the source of truth here, mirrored verbatim
//! in `assertions/witness/defense_gate.toml`.  The binary's
//! `--print-anchors` mode dumps the registry as JSON for audit.
//!
//! Refs: ONE SHOT v2.0 §3 Phase C · §5 falsification table · trios#265.
//! Trinity anchor: `phi^2 + phi^-2 = 3` · DOI 10.5281/zenodo.19227877.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Pre-registered floors (Phase C §3 of ONE SHOT v2.0)
// ---------------------------------------------------------------------------

/// Examiner pack must have at least this many sectioned units
/// (`\section` / `\subsection` / `\subsubsection`) — proxy for "50 pp".
pub const EXAMINER_MIN_SECTIONS: usize = 50;

/// Q&A document must have at least this many Q/A blocks.
pub const QA_MIN_PAIRS: usize = 30;

/// Beamer slides document must have at least this many frame blocks.
pub const SLIDES_MIN_FRAMES: usize = 30;

/// Default defense directory relative to the repository root.
pub const DEFAULT_DEFENSE_DIR: &str = "docs/phd/defense";

/// Filename of the examiner pack inside the defense directory.
pub const EXAMINER_PACK_FILENAME: &str = "examiner-pack.tex";

/// Filename of the Q&A pack inside the defense directory.
pub const QA_FILENAME: &str = "qa.tex";

/// Filename of the Beamer slides inside the defense directory.
pub const SLIDES_FILENAME: &str = "slides.tex";

// Compile-time sanity: floors strictly positive (a zero floor would
// make the gate a no-op and break R8 falsifiability).
const _: () = assert!(
    EXAMINER_MIN_SECTIONS >= 1,
    "EXAMINER_MIN_SECTIONS must be >= 1 (R8)"
);
const _: () = assert!(QA_MIN_PAIRS >= 1, "QA_MIN_PAIRS must be >= 1 (R8)");
const _: () = assert!(SLIDES_MIN_FRAMES >= 1, "SLIDES_MIN_FRAMES must be >= 1 (R8)");

// ---------------------------------------------------------------------------
// Error model — exit codes 100..=104 (disjoint per coq-runtime-invariants v1.1)
// ---------------------------------------------------------------------------

/// Reasons the defense gate refuses to admit a defense package.
///
/// Exit codes 100..=104 are disjoint from L7 (0..=10), L15 (21..=30),
/// L-h4 (50..=53), LT (60..=63), LA (70..=74), citetheorem_audit
/// (80..=83), and merge_order_gate (90..=93).
#[derive(Debug, Error)]
pub enum DefenseGateError {
    /// `--defense-dir` could not be opened or is not a directory.
    #[error("defense-gate: defense dir unreadable ({path}): {detail}")]
    DefenseDirError {
        /// Path that failed.
        path: String,
        /// Human-readable cause.
        detail: String,
    },

    /// One of the three required defense files is missing on disk.
    #[error("defense-gate: required defense file missing: {path}")]
    MissingDefenseFile {
        /// Absolute or repo-relative path.
        path: String,
    },

    /// Examiner pack section count below floor.
    #[error(
        "defense-gate: examiner pack {path} has {actual} sectioned units, \
         need at least {floor}"
    )]
    ExaminerPackTooShort {
        /// Path of the offending file.
        path: String,
        /// Sections found.
        actual: usize,
        /// Floor (mirror of [`EXAMINER_MIN_SECTIONS`]).
        floor: usize,
    },

    /// Q&A pair count below floor.
    #[error(
        "defense-gate: qa pack {path} has {actual} Q/A pairs, \
         need at least {floor}"
    )]
    QaPairsBelowFloor {
        /// Path of the offending file.
        path: String,
        /// Pairs found.
        actual: usize,
        /// Floor (mirror of [`QA_MIN_PAIRS`]).
        floor: usize,
    },

    /// Slides frame count below floor.
    #[error(
        "defense-gate: slides {path} carry {actual} frames, \
         need at least {floor}"
    )]
    SlidesBelowFloor {
        /// Path of the offending file.
        path: String,
        /// Frames found.
        actual: usize,
        /// Floor (mirror of [`SLIDES_MIN_FRAMES`]).
        floor: usize,
    },
}

impl DefenseGateError {
    /// Process exit code paired with this error variant.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::DefenseDirError { .. } => 100,
            Self::ExaminerPackTooShort { .. } => 101,
            Self::QaPairsBelowFloor { .. } => 102,
            Self::SlidesBelowFloor { .. } => 103,
            Self::MissingDefenseFile { .. } => 104,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Audit summary returned in report mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DefenseReport {
    /// Defense directory inspected.
    pub defense_dir: String,
    /// Examiner-pack section count (None if file missing).
    pub examiner_sections: Option<usize>,
    /// Q&A pair count (None if file missing).
    pub qa_pairs: Option<usize>,
    /// Slides frame count (None if file missing).
    pub slides_frames: Option<usize>,
    /// Floors (mirror of the three constants).
    pub floors: ReportFloors,
    /// Violations found, in stable order.
    pub violations: Vec<String>,
    /// `true` iff strict mode would have admitted (zero violations and
    /// all three files present).
    pub passes_strict: bool,
}

/// Floors snapshot embedded in [`DefenseReport`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportFloors {
    /// Mirror of [`EXAMINER_MIN_SECTIONS`].
    pub examiner_min_sections: usize,
    /// Mirror of [`QA_MIN_PAIRS`].
    pub qa_min_pairs: usize,
    /// Mirror of [`SLIDES_MIN_FRAMES`].
    pub slides_min_frames: usize,
}

impl ReportFloors {
    /// Build the floors snapshot from the const registry.
    pub fn from_registry() -> Self {
        Self {
            examiner_min_sections: EXAMINER_MIN_SECTIONS,
            qa_min_pairs: QA_MIN_PAIRS,
            slides_min_frames: SLIDES_MIN_FRAMES,
        }
    }
}

/// L-R14 anchor metadata for `--print-anchors`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DefenseAnchors {
    /// Examiner-pack floor (sections).
    pub examiner_min_sections: usize,
    /// Q&A floor (pairs).
    pub qa_min_pairs: usize,
    /// Slides floor (frames).
    pub slides_min_frames: usize,
    /// Default defense dir.
    pub default_defense_dir: String,
    /// Examiner pack filename.
    pub examiner_pack_filename: String,
    /// Q&A filename.
    pub qa_filename: String,
    /// Slides filename.
    pub slides_filename: String,
    /// Trinity identity.
    pub trinity_identity: String,
    /// Zenodo DOI.
    pub zenodo_doi: String,
}

impl DefenseAnchors {
    /// Snapshot the Rust source of truth.
    pub fn from_registry() -> Self {
        Self {
            examiner_min_sections: EXAMINER_MIN_SECTIONS,
            qa_min_pairs: QA_MIN_PAIRS,
            slides_min_frames: SLIDES_MIN_FRAMES,
            default_defense_dir: DEFAULT_DEFENSE_DIR.to_string(),
            examiner_pack_filename: EXAMINER_PACK_FILENAME.to_string(),
            qa_filename: QA_FILENAME.to_string(),
            slides_filename: SLIDES_FILENAME.to_string(),
            trinity_identity: "phi^2 + phi^-2 = 3".to_string(),
            zenodo_doi: "10.5281/zenodo.19227877".to_string(),
        }
    }
}

/// Run the strict gate against the on-disk defense directory.
///
/// Returns `Ok(())` iff all three files are present and meet their
/// floors.  The first failure short-circuits to `Err` carrying its
/// own exit code via [`DefenseGateError::exit_code`].
pub fn audit_strict(defense_dir: &Path) -> Result<(), DefenseGateError> {
    apply_strict_checks(defense_dir, &load_or_collect(defense_dir)?)
}

/// Run report mode — never returns `Err` for floor breaches; instead
/// records them in `violations`.  Still bubbles up directory I/O errors
/// (`DefenseDirError`) so a misconfigured invocation cannot silently
/// pass.
pub fn audit_report(defense_dir: &Path) -> Result<DefenseReport, DefenseGateError> {
    let collected = load_or_collect(defense_dir)?;

    let mut violations: Vec<String> = Vec::new();

    match collected.examiner_sections {
        None => violations.push(format!(
            "missing examiner pack: {}",
            defense_dir
                .join(EXAMINER_PACK_FILENAME)
                .display()
        )),
        Some(n) if n < EXAMINER_MIN_SECTIONS => violations.push(format!(
            "examiner pack {} sections < floor {}",
            n, EXAMINER_MIN_SECTIONS
        )),
        Some(_) => {}
    }
    match collected.qa_pairs {
        None => violations.push(format!(
            "missing qa pack: {}",
            defense_dir.join(QA_FILENAME).display()
        )),
        Some(n) if n < QA_MIN_PAIRS => {
            violations.push(format!("qa pairs {} < floor {}", n, QA_MIN_PAIRS))
        }
        Some(_) => {}
    }
    match collected.slides_frames {
        None => violations.push(format!(
            "missing slides: {}",
            defense_dir.join(SLIDES_FILENAME).display()
        )),
        Some(n) if n < SLIDES_MIN_FRAMES => violations.push(format!(
            "slides frames {} < floor {}",
            n, SLIDES_MIN_FRAMES
        )),
        Some(_) => {}
    }

    let passes_strict = violations.is_empty();

    Ok(DefenseReport {
        defense_dir: defense_dir.display().to_string(),
        examiner_sections: collected.examiner_sections,
        qa_pairs: collected.qa_pairs,
        slides_frames: collected.slides_frames,
        floors: ReportFloors::from_registry(),
        violations,
        passes_strict,
    })
}

/// Render a [`DefenseReport`] as pretty JSON.
pub fn render_report_json(report: &DefenseReport) -> String {
    serde_json::to_string_pretty(report).expect("DefenseReport always serialises")
}

// ---------------------------------------------------------------------------
// Internal collection layer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Collected {
    examiner_sections: Option<usize>,
    qa_pairs: Option<usize>,
    slides_frames: Option<usize>,
}

fn load_or_collect(defense_dir: &Path) -> Result<Collected, DefenseGateError> {
    let meta = fs::metadata(defense_dir).map_err(|e| DefenseGateError::DefenseDirError {
        path: defense_dir.display().to_string(),
        detail: e.to_string(),
    })?;
    if !meta.is_dir() {
        return Err(DefenseGateError::DefenseDirError {
            path: defense_dir.display().to_string(),
            detail: "not a directory".to_string(),
        });
    }

    let examiner_sections =
        read_optional(&defense_dir.join(EXAMINER_PACK_FILENAME))?.map(|src| count_sections(&src));
    let qa_pairs = read_optional(&defense_dir.join(QA_FILENAME))?.map(|src| count_qa_pairs(&src));
    let slides_frames =
        read_optional(&defense_dir.join(SLIDES_FILENAME))?.map(|src| count_frames(&src));

    Ok(Collected {
        examiner_sections,
        qa_pairs,
        slides_frames,
    })
}

fn read_optional(path: &Path) -> Result<Option<String>, DefenseGateError> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(DefenseGateError::DefenseDirError {
            path: path.display().to_string(),
            detail: e.to_string(),
        }),
    }
}

fn apply_strict_checks(
    defense_dir: &Path,
    collected: &Collected,
) -> Result<(), DefenseGateError> {
    // Missingness wins over floor violations — if the file isn't there,
    // counting zero is meaningless.
    let examiner_path = defense_dir.join(EXAMINER_PACK_FILENAME);
    let qa_path = defense_dir.join(QA_FILENAME);
    let slides_path = defense_dir.join(SLIDES_FILENAME);

    let n_examiner = collected
        .examiner_sections
        .ok_or_else(|| DefenseGateError::MissingDefenseFile {
            path: examiner_path.display().to_string(),
        })?;
    let n_qa = collected
        .qa_pairs
        .ok_or_else(|| DefenseGateError::MissingDefenseFile {
            path: qa_path.display().to_string(),
        })?;
    let n_slides =
        collected
            .slides_frames
            .ok_or_else(|| DefenseGateError::MissingDefenseFile {
                path: slides_path.display().to_string(),
            })?;

    if n_examiner < EXAMINER_MIN_SECTIONS {
        return Err(DefenseGateError::ExaminerPackTooShort {
            path: examiner_path.display().to_string(),
            actual: n_examiner,
            floor: EXAMINER_MIN_SECTIONS,
        });
    }
    if n_qa < QA_MIN_PAIRS {
        return Err(DefenseGateError::QaPairsBelowFloor {
            path: qa_path.display().to_string(),
            actual: n_qa,
            floor: QA_MIN_PAIRS,
        });
    }
    if n_slides < SLIDES_MIN_FRAMES {
        return Err(DefenseGateError::SlidesBelowFloor {
            path: slides_path.display().to_string(),
            actual: n_slides,
            floor: SLIDES_MIN_FRAMES,
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Pure counters — exposed as helper functions for unit testing.
// ---------------------------------------------------------------------------

/// Count `\section{`, `\subsection{`, `\subsubsection{` occurrences in
/// LaTeX source, ignoring lines that start with `%` (comments).
///
/// "Sectioned units" is the structural proxy for "50 pp" used by the
/// gate — this avoids shelling out to `pdfinfo` (R1: Rust-only).
pub fn count_sections(src: &str) -> usize {
    let mut total = 0usize;
    for line in src.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('%') {
            continue;
        }
        // We count substring occurrences, not full-line matches, so a
        // single line with two `\subsection{...}` calls still counts twice.
        total += count_substr(trimmed, "\\section{");
        total += count_substr(trimmed, "\\subsection{");
        total += count_substr(trimmed, "\\subsubsection{");
    }
    total
}

/// Count Q/A pairs.  Three accepted markers (any one suffices per item):
///
/// - `\question{...}` (Lee/GVSU style command)
/// - `\qaitem{...}`   (custom defense-pack macro)
/// - `\textbf{Q`      (visible Q/A boldface)
///
/// To avoid double-counting when a document mixes styles, the function
/// returns the **maximum** of the three counts.
pub fn count_qa_pairs(src: &str) -> usize {
    let mut question_cmd = 0usize;
    let mut qaitem_cmd = 0usize;
    let mut textbf_q = 0usize;
    for line in src.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('%') {
            continue;
        }
        question_cmd += count_substr(trimmed, "\\question{");
        qaitem_cmd += count_substr(trimmed, "\\qaitem{");
        textbf_q += count_substr(trimmed, "\\textbf{Q");
    }
    question_cmd.max(qaitem_cmd).max(textbf_q)
}

/// Count Beamer frame blocks: `\begin{frame}` (with optional `[...]`)
/// or self-closing `\frame{...}`.  Lines starting with `%` are skipped.
pub fn count_frames(src: &str) -> usize {
    let mut total = 0usize;
    for line in src.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('%') {
            continue;
        }
        total += count_substr(trimmed, "\\begin{frame}");
        total += count_substr(trimmed, "\\frame{");
    }
    total
}

fn count_substr(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0usize;
    let mut idx = 0usize;
    while let Some(found) = haystack[idx..].find(needle) {
        count += 1;
        idx += found + needle.len();
        if idx >= haystack.len() {
            break;
        }
    }
    count
}

// ---------------------------------------------------------------------------
// Path helper
// ---------------------------------------------------------------------------

/// Resolve the path the binary should audit, given an optional
/// `--defense-dir` override.
pub fn resolve_defense_dir(arg: Option<&str>) -> PathBuf {
    PathBuf::from(arg.unwrap_or(DEFAULT_DEFENSE_DIR))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // --------- helpers ---------

    fn make_dir() -> TempDir {
        TempDir::new().expect("tempdir")
    }

    fn write_file(dir: &Path, name: &str, body: &str) {
        fs::write(dir.join(name), body).expect("write");
    }

    fn examiner_with_n(n: usize) -> String {
        let mut s = String::new();
        s.push_str("\\documentclass{article}\n");
        for i in 0..n {
            s.push_str(&format!("\\section{{Q{}}}\nbody\n", i));
        }
        s
    }

    fn qa_with_n(n: usize) -> String {
        let mut s = String::new();
        for i in 0..n {
            s.push_str(&format!("\\textbf{{Q{}}}: question\n\\textbf{{A}}: answer\n", i));
        }
        s
    }

    fn slides_with_n(n: usize) -> String {
        let mut s = String::new();
        for i in 0..n {
            s.push_str(&format!(
                "\\begin{{frame}}\nslide {}\n\\end{{frame}}\n",
                i
            ));
        }
        s
    }

    // --------- happy path (4) ---------

    #[test]
    fn admits_full_package() {
        let d = make_dir();
        write_file(d.path(), EXAMINER_PACK_FILENAME, &examiner_with_n(50));
        write_file(d.path(), QA_FILENAME, &qa_with_n(30));
        write_file(d.path(), SLIDES_FILENAME, &slides_with_n(30));
        assert!(audit_strict(d.path()).is_ok());
    }

    #[test]
    fn admits_overshoot_package() {
        let d = make_dir();
        write_file(d.path(), EXAMINER_PACK_FILENAME, &examiner_with_n(120));
        write_file(d.path(), QA_FILENAME, &qa_with_n(60));
        write_file(d.path(), SLIDES_FILENAME, &slides_with_n(45));
        let report = audit_report(d.path()).expect("report");
        assert!(report.passes_strict);
        assert_eq!(report.examiner_sections, Some(120));
        assert_eq!(report.qa_pairs, Some(60));
        assert_eq!(report.slides_frames, Some(45));
        assert!(report.violations.is_empty());
    }

    #[test]
    fn admits_with_mixed_section_levels() {
        // 25 \section + 25 \subsection + 25 \subsubsection = 75 ≥ 50.
        let mut s = String::new();
        for i in 0..25 {
            s.push_str(&format!("\\section{{S{i}}}\n"));
            s.push_str(&format!("\\subsection{{T{i}}}\n"));
            s.push_str(&format!("\\subsubsection{{U{i}}}\n"));
        }
        let d = make_dir();
        write_file(d.path(), EXAMINER_PACK_FILENAME, &s);
        write_file(d.path(), QA_FILENAME, &qa_with_n(30));
        write_file(d.path(), SLIDES_FILENAME, &slides_with_n(30));
        assert!(audit_strict(d.path()).is_ok());
    }

    #[test]
    fn admits_qa_via_qaitem_macro() {
        let d = make_dir();
        let mut qa = String::new();
        for i in 0..30 {
            qa.push_str(&format!("\\qaitem{{Q{i}}}{{A{i}}}\n"));
        }
        write_file(d.path(), EXAMINER_PACK_FILENAME, &examiner_with_n(50));
        write_file(d.path(), QA_FILENAME, &qa);
        write_file(d.path(), SLIDES_FILENAME, &slides_with_n(30));
        assert!(audit_strict(d.path()).is_ok());
    }

    // --------- falsification (5) ---------

    #[test]
    fn falsify_defense_dir_error_missing_dir() {
        let result = audit_strict(Path::new("/nonexistent/path/abc/xyz"));
        let err = result.expect_err("expected DefenseDirError");
        assert!(matches!(err, DefenseGateError::DefenseDirError { .. }));
        assert_eq!(err.exit_code(), 100);
    }

    #[test]
    fn falsify_defense_dir_error_is_a_file() {
        let d = make_dir();
        let path = d.path().join("not-a-dir");
        fs::write(&path, "hi").unwrap();
        let err = audit_strict(&path).expect_err("expected DefenseDirError");
        assert!(matches!(err, DefenseGateError::DefenseDirError { .. }));
        assert_eq!(err.exit_code(), 100);
    }

    #[test]
    fn falsify_examiner_pack_too_short() {
        let d = make_dir();
        write_file(d.path(), EXAMINER_PACK_FILENAME, &examiner_with_n(10));
        write_file(d.path(), QA_FILENAME, &qa_with_n(30));
        write_file(d.path(), SLIDES_FILENAME, &slides_with_n(30));
        let err = audit_strict(d.path()).expect_err("expected ExaminerPackTooShort");
        match err {
            DefenseGateError::ExaminerPackTooShort {
                actual, floor, ..
            } => {
                assert_eq!(actual, 10);
                assert_eq!(floor, EXAMINER_MIN_SECTIONS);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn falsify_qa_pairs_below_floor() {
        let d = make_dir();
        write_file(d.path(), EXAMINER_PACK_FILENAME, &examiner_with_n(50));
        write_file(d.path(), QA_FILENAME, &qa_with_n(5));
        write_file(d.path(), SLIDES_FILENAME, &slides_with_n(30));
        let err = audit_strict(d.path()).expect_err("expected QaPairsBelowFloor");
        match err {
            DefenseGateError::QaPairsBelowFloor { actual, floor, .. } => {
                assert_eq!(actual, 5);
                assert_eq!(floor, QA_MIN_PAIRS);
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(102, DefenseGateError::QaPairsBelowFloor {
            path: "x".into(), actual: 0, floor: 1,
        }.exit_code());
    }

    #[test]
    fn falsify_slides_below_floor() {
        let d = make_dir();
        write_file(d.path(), EXAMINER_PACK_FILENAME, &examiner_with_n(50));
        write_file(d.path(), QA_FILENAME, &qa_with_n(30));
        write_file(d.path(), SLIDES_FILENAME, &slides_with_n(7));
        let err = audit_strict(d.path()).expect_err("expected SlidesBelowFloor");
        match err {
            DefenseGateError::SlidesBelowFloor { actual, floor, .. } => {
                assert_eq!(actual, 7);
                assert_eq!(floor, SLIDES_MIN_FRAMES);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn falsify_missing_defense_file() {
        let d = make_dir();
        // Only examiner present — qa.tex and slides.tex absent.
        write_file(d.path(), EXAMINER_PACK_FILENAME, &examiner_with_n(50));
        let err = audit_strict(d.path()).expect_err("expected MissingDefenseFile");
        assert!(matches!(err, DefenseGateError::MissingDefenseFile { .. }));
        assert_eq!(err.exit_code(), 104);
    }

    // --------- helper invariants (6) ---------

    #[test]
    fn count_sections_skips_comment_lines() {
        let src = "% \\section{ignored}\n\\section{real}\n";
        assert_eq!(count_sections(src), 1);
    }

    #[test]
    fn count_sections_handles_multi_per_line() {
        let src = "\\section{a}\\subsection{b}\\subsubsection{c}\n";
        assert_eq!(count_sections(src), 3);
    }

    #[test]
    fn count_qa_pairs_returns_max_of_styles() {
        // 5 \question + 7 \qaitem + 9 \textbf{Q -> max = 9.
        let mut s = String::new();
        for _ in 0..5 {
            s.push_str("\\question{q}\n");
        }
        for _ in 0..7 {
            s.push_str("\\qaitem{q}{a}\n");
        }
        for _ in 0..9 {
            s.push_str("\\textbf{Q}: hi\n");
        }
        assert_eq!(count_qa_pairs(&s), 9);
    }

    #[test]
    fn count_frames_handles_both_styles() {
        let src = "\\begin{frame}\n\\end{frame}\n\\frame{single}\n";
        assert_eq!(count_frames(src), 2);
    }

    #[test]
    fn count_substr_does_not_overlap() {
        // "aaa" contains "aa" twice with overlap, but our counter is
        // non-overlapping by design (advance by needle length).
        assert_eq!(count_substr("aaa", "aa"), 1);
        assert_eq!(count_substr("aaaa", "aa"), 2);
    }

    #[test]
    fn report_round_trips_through_json() {
        let report = DefenseReport {
            defense_dir: "docs/phd/defense".into(),
            examiner_sections: Some(50),
            qa_pairs: Some(30),
            slides_frames: Some(30),
            floors: ReportFloors::from_registry(),
            violations: vec![],
            passes_strict: true,
        };
        let s = render_report_json(&report);
        let back: DefenseReport = serde_json::from_str(&s).expect("round trip");
        assert_eq!(report, back);
    }

    #[test]
    fn floors_are_pre_registered_constants() {
        assert_eq!(EXAMINER_MIN_SECTIONS, 50);
        assert_eq!(QA_MIN_PAIRS, 30);
        assert_eq!(SLIDES_MIN_FRAMES, 30);
    }

    #[test]
    fn anchors_emit_full_registry() {
        let a = DefenseAnchors::from_registry();
        assert_eq!(a.examiner_min_sections, 50);
        assert_eq!(a.qa_min_pairs, 30);
        assert_eq!(a.slides_min_frames, 30);
        assert_eq!(a.default_defense_dir, "docs/phd/defense");
        assert_eq!(a.examiner_pack_filename, "examiner-pack.tex");
        assert_eq!(a.qa_filename, "qa.tex");
        assert_eq!(a.slides_filename, "slides.tex");
        assert!(a.zenodo_doi.contains("19227877"));
    }

    #[test]
    fn report_mode_aggregates_violations() {
        let d = make_dir();
        write_file(d.path(), EXAMINER_PACK_FILENAME, &examiner_with_n(10));
        write_file(d.path(), QA_FILENAME, &qa_with_n(5));
        // slides missing entirely
        let report = audit_report(d.path()).expect("report");
        assert!(!report.passes_strict);
        // Three violations: examiner short, qa short, slides missing.
        assert_eq!(report.violations.len(), 3);
        assert_eq!(report.examiner_sections, Some(10));
        assert_eq!(report.qa_pairs, Some(5));
        assert_eq!(report.slides_frames, None);
    }

    #[test]
    fn resolve_defense_dir_uses_default_when_none() {
        let p = resolve_defense_dir(None);
        assert_eq!(p, PathBuf::from(DEFAULT_DEFENSE_DIR));
    }

    #[test]
    fn resolve_defense_dir_honours_override() {
        let p = resolve_defense_dir(Some("/tmp/xyz"));
        assert_eq!(p, PathBuf::from("/tmp/xyz"));
    }
}
