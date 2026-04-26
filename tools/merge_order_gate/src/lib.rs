//! `merge_order_gate` — Phase A merge-order witness library.
//!
//! Pure structural witness for the «Flos Aureus» PhD monograph
//! (trios#265 ONE SHOT v2.0 §5 row 1):
//!
//! > *exits 1 if a chapter PR merges before #288/#289/#294*
//!
//! ## What this enforces
//!
//! Phase A of the ONE SHOT prescribes a strict topological merge order:
//!
//! 1. `feat/phd-appendix-F-restore` (PR #288) — restores appendix F
//!    Coq citation map; **every chapter `\citetheorem` reference depends
//!    on this**.
//! 2. `feat/phd-lf-frontmatter` (PR #289) — frontmatter scaffold.
//! 3. `feat/phd-lb-springer`    (PR #294) — bibliography balance.
//! 4. *…then* chapter PRs `feat/phd-ch*` may merge.
//!
//! This crate inspects a `git log` first-parent transcript and verifies
//! that for every chapter-merge commit, all three prerequisite merges
//! appear at strictly earlier indices.
//!
//! On `main` at the time this crate landed, **zero chapter PRs have
//! merged** — the Phase A merge wave is still queue-bot-blocked.  The
//! gate is therefore vacuously ADMIT(0) on the live transcript; it
//! becomes load-bearing the instant the first chapter merge lands.
//!
//! ## R1 / R6 / R10 compliance
//!
//! - R1: pure Rust.  Replaces the `.sh` placeholder named in §5 row 1.
//! - R6: greenfield `tools/merge_order_gate/`.
//! - R10: ships in one atomic commit per the ONE SHOT contract.
//!
//! ## R8 falsification witnesses
//!
//! Each rejection variant of [`MergeOrderError`] is paired with a
//! `falsify_*` unit test that constructs the failing transcript and
//! asserts the corresponding exit code.
//!
//! ## L-R14 traceability
//!
//! The pre-registered prerequisite branch list is a constant here,
//! mirrored verbatim in `assertions/witness/merge_order_gate.toml`;
//! the binary's `--print-anchors` mode dumps the registry as JSON for
//! audit.  Any change to the prerequisite list MUST sibling-commit a
//! matching change to the manifest (R10).
//!
//! Refs: ONE SHOT v2.0 §3 Phase A · §5 falsification table · trios#265.
//! Trinity anchor: `phi^2 + phi^-2 = 3` · DOI 10.5281/zenodo.19227877.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Pre-registered prerequisite branches (Phase A §3 of ONE SHOT v2.0)
// ---------------------------------------------------------------------------

/// Branches whose merge commits MUST appear in the first-parent history
/// before any chapter merge (`feat/phd-ch*`).
///
/// Order in the slice mirrors §3 Phase A of ONE SHOT v2.0; the gate
/// itself is set-based — only presence-before-chapter matters.
pub const PHASE_A_PREREQS: &[&str] = &[
    "feat/phd-appendix-F-restore",
    "feat/phd-lf-frontmatter",
    "feat/phd-lb-springer",
];

/// Branch-name prefix that identifies a regulated chapter PR.
pub const CHAPTER_PREFIX: &str = "feat/phd-ch";

// Compile-time sanity: prerequisite list non-empty (an empty list would
// make the gate a no-op and break R8).
const _: () = assert!(
    !PHASE_A_PREREQS.is_empty(),
    "PHASE_A_PREREQS must be non-empty (R8: gate must reject empty registries)"
);

// ---------------------------------------------------------------------------
// Error model — exit codes 90..=93 (disjoint per coq-runtime-invariants v1.1)
// ---------------------------------------------------------------------------

/// Reasons the merge-order gate refuses to admit the transcript.
///
/// Exit codes 90..=93 are disjoint from L7 (0..=10), L15 (21..=30),
/// L-h4 (50..=53), LT (60..=63), LA (70..=74), and citetheorem_audit
/// (80..=83).
#[derive(Debug, Error)]
pub enum MergeOrderError {
    /// `--git-log` file could not be read.
    #[error("merge-order-gate: git-log unreadable ({path}): {detail}")]
    GitLogReadError {
        /// Path that failed to load.
        path: String,
        /// Human-readable cause.
        detail: String,
    },

    /// A line of the transcript looked like a merge commit but did not
    /// parse — almost certainly a malformed `--git-log` snapshot.
    #[error("merge-order-gate: malformed merge entry on line {line_no}: {raw:?}")]
    MalformedMergeEntry {
        /// 1-indexed line number where parsing failed.
        line_no: usize,
        /// Verbatim line.
        raw: String,
    },

    /// A chapter PR merge appears at index `chapter_idx` but at least
    /// one Phase A prerequisite is missing from preceding indices.
    #[error(
        "merge-order-gate: chapter merge {chapter_branch} at index {chapter_idx} \
         precedes {missing_n} prerequisite(s): {missing:?}"
    )]
    ChapterBeforePrereq {
        /// Branch name of the offending chapter merge.
        chapter_branch: String,
        /// Index (1-based) of the chapter merge in the first-parent log.
        chapter_idx: usize,
        /// Count of prerequisites missing strictly before `chapter_idx`.
        missing_n: usize,
        /// Names of the missing prerequisites (sorted).
        missing: Vec<String>,
    },

    /// A pre-registered Phase A prerequisite never appears in the
    /// transcript at all, even though chapter merges DO appear.  This
    /// implies the merge wave bypassed the registry entirely.
    #[error(
        "merge-order-gate: chapter merges present but prerequisite {prereq} never merged"
    )]
    PrereqNeverMerged {
        /// Branch name of the missing prerequisite.
        prereq: String,
    },
}

impl MergeOrderError {
    /// Stable exit-code mapping (disjoint from sibling lanes).
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::GitLogReadError { .. } => 90,
            Self::MalformedMergeEntry { .. } => 91,
            Self::ChapterBeforePrereq { .. } => 92,
            Self::PrereqNeverMerged { .. } => 93,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// One row in the parsed first-parent transcript.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MergeRow {
    /// 1-indexed index in the original transcript.
    pub idx: usize,
    /// Short SHA (or full SHA — the gate is SHA-opaque, only branch matters).
    pub sha: String,
    /// Source branch name extracted from the merge subject.
    pub branch: String,
    /// `true` iff `branch.starts_with(CHAPTER_PREFIX)`.
    pub is_chapter: bool,
    /// `true` iff `branch` matches one of [`PHASE_A_PREREQS`].
    pub is_prereq: bool,
}

/// Aggregated audit output (also emitted in `--report` mode).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderReport {
    /// Parsed transcript rows (chronological).
    pub rows: Vec<MergeRow>,
    /// Branches recognised as Phase A prerequisites in this transcript.
    pub prereqs_seen: Vec<String>,
    /// Chapter merges seen in this transcript.
    pub chapter_merges: Vec<MergeRow>,
    /// Per-chapter ordering violations (empty iff `all_pass`).
    pub violations: Vec<OrderViolation>,
    /// `true` iff every chapter merge has all prerequisites at earlier indices.
    pub all_pass: bool,
}

/// One ordering violation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderViolation {
    /// Chapter merge that violated the order.
    pub chapter: MergeRow,
    /// Prerequisites missing strictly before `chapter.idx` (sorted).
    pub missing: Vec<String>,
}

/// Strict-mode entry point.
///
/// Reads a first-parent `git log` snapshot and returns `Ok(report)` iff
/// every chapter PR merge has all [`PHASE_A_PREREQS`] at strictly earlier
/// transcript indices.
pub fn audit_strict(git_log_path: &Path) -> Result<OrderReport, MergeOrderError> {
    let report = audit_report(git_log_path)?;
    apply_strict_checks(report)
}

/// Report-mode entry point: parses + diagnoses but never errors on
/// ordering violations — only on I/O / parse failures.
pub fn audit_report(git_log_path: &Path) -> Result<OrderReport, MergeOrderError> {
    let body = fs::read_to_string(git_log_path).map_err(|e| MergeOrderError::GitLogReadError {
        path: git_log_path.display().to_string(),
        detail: e.to_string(),
    })?;
    parse_and_diagnose(&body)
}

/// Parse a transcript already loaded into memory (report mode — never
/// errors on ordering violations).  Useful for unit tests and callers
/// that already have a `git log` capture in a string.
pub fn audit_str(transcript: &str) -> Result<OrderReport, MergeOrderError> {
    parse_and_diagnose(transcript)
}

/// Strict-mode variant of [`audit_str`].  Surfaces ordering violations
/// as `Err(MergeOrderError::ChapterBeforePrereq | PrereqNeverMerged)`.
pub fn audit_str_strict(transcript: &str) -> Result<OrderReport, MergeOrderError> {
    let report = parse_and_diagnose(transcript)?;
    apply_strict_checks(report)
}

fn apply_strict_checks(report: OrderReport) -> Result<OrderReport, MergeOrderError> {
    if !report.chapter_merges.is_empty() {
        let seen: BTreeSet<&str> = report.prereqs_seen.iter().map(String::as_str).collect();
        for prereq in PHASE_A_PREREQS {
            if !seen.contains(prereq) {
                return Err(MergeOrderError::PrereqNeverMerged {
                    prereq: (*prereq).to_string(),
                });
            }
        }
    }
    if let Some(violation) = report.violations.first() {
        return Err(MergeOrderError::ChapterBeforePrereq {
            chapter_branch: violation.chapter.branch.clone(),
            chapter_idx: violation.chapter.idx,
            missing_n: violation.missing.len(),
            missing: violation.missing.clone(),
        });
    }
    Ok(report)
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn parse_and_diagnose(transcript: &str) -> Result<OrderReport, MergeOrderError> {
    let rows = parse_transcript(transcript)?;

    let prereqs: BTreeSet<&str> = PHASE_A_PREREQS.iter().copied().collect();
    let mut prereqs_seen_set: BTreeSet<String> = BTreeSet::new();
    let mut chapter_merges = Vec::new();

    for row in &rows {
        if row.is_prereq {
            prereqs_seen_set.insert(row.branch.clone());
        }
        if row.is_chapter {
            chapter_merges.push(row.clone());
        }
    }

    let mut violations = Vec::new();
    for chapter in &chapter_merges {
        let merged_before: BTreeSet<&str> = rows
            .iter()
            .take(chapter.idx.saturating_sub(1))
            .filter(|r| r.is_prereq)
            .map(|r| r.branch.as_str())
            .collect();
        let missing: Vec<String> = prereqs
            .iter()
            .filter(|p| !merged_before.contains(*p))
            .map(|p| (*p).to_string())
            .collect();
        if !missing.is_empty() {
            violations.push(OrderViolation {
                chapter: chapter.clone(),
                missing,
            });
        }
    }

    let all_pass = violations.is_empty()
        && (chapter_merges.is_empty()
            || PHASE_A_PREREQS
                .iter()
                .all(|p| prereqs_seen_set.contains(*p)));

    Ok(OrderReport {
        rows,
        prereqs_seen: prereqs_seen_set.into_iter().collect(),
        chapter_merges,
        violations,
        all_pass,
    })
}

/// Parse `git log --merges --first-parent --pretty=format:'%h %s'` style
/// output.
///
/// Each line MUST match: `<sha><WS+>Merge pull request #<N> from
/// <slug>/<branch>` or the more lenient `<sha><WS+>... from
/// <slug>/<branch>` form.  Empty lines and lines that are not
/// merge-commit subjects are skipped silently.
///
/// Lines that look like a merge subject (`from <slug>/<branch>`) but do
/// NOT carry a SHA prefix (i.e. the `git log` was malformed) raise
/// [`MergeOrderError::MalformedMergeEntry`] on the first offender.
pub(crate) fn parse_transcript(body: &str) -> Result<Vec<MergeRow>, MergeOrderError> {
    let mut out = Vec::new();
    for (i, raw) in body.lines().enumerate() {
        let line = raw.trim_end();
        if line.is_empty() {
            continue;
        }

        // Heuristic: a merge subject contains " from " followed by a slug/branch.
        let from_idx = match line.find(" from ") {
            Some(i) => i,
            None => continue, // not a merge subject — skip
        };

        // Extract SHA: first whitespace-delimited token.
        let mut tokens = line.splitn(2, char::is_whitespace);
        let sha = tokens.next().unwrap_or("").trim();
        let _rest = tokens.next().unwrap_or("");
        if sha.is_empty() || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(MergeOrderError::MalformedMergeEntry {
                line_no: i + 1,
                raw: line.to_string(),
            });
        }

        // Branch lives after " from <slug>/" and runs until the next
        // whitespace, end-of-line, or trailing punctuation.
        let after_from = &line[from_idx + " from ".len()..];
        let after_slash = match after_from.find('/') {
            Some(idx) => &after_from[idx + 1..],
            None => continue, // malformed merge subject — best-effort skip
        };
        let branch_end = after_slash
            .find(|c: char| c.is_whitespace())
            .unwrap_or(after_slash.len());
        let branch = after_slash[..branch_end]
            .trim_end_matches(['.', ',', ';', ')'])
            .to_string();
        if branch.is_empty() {
            continue;
        }

        let is_chapter = branch.starts_with(CHAPTER_PREFIX);
        let is_prereq = PHASE_A_PREREQS.iter().any(|p| *p == branch);

        out.push(MergeRow {
            idx: out.len() + 1,
            sha: sha.to_string(),
            branch,
            is_chapter,
            is_prereq,
        });
    }
    Ok(out)
}

/// Convenience helper for downstream tools (CI summary scripts):
/// load the report and convert it to canonical pretty-printed JSON.
pub fn render_report_json(report: &OrderReport) -> Result<String, std::io::Error> {
    serde_json::to_string_pretty(report).map_err(std::io::Error::other)
}

// ---------------------------------------------------------------------------
// Tests — falsification witnesses (one per MergeOrderError variant) plus
// happy-path coverage.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper: build a synthetic `git log` transcript line.
    fn merge_line(sha: &str, pr: u32, slug: &str, branch: &str) -> String {
        format!(
            "{sha} Merge pull request #{pr} from {slug}/{branch}\n",
            sha = sha,
            pr = pr,
            slug = slug,
            branch = branch,
        )
    }

    fn write_log(dir: &Path, body: &str) -> PathBuf {
        let p = dir.join("git.log");
        fs::write(&p, body).unwrap();
        p
    }

    // ---------- happy paths ------------------------------------------------

    #[test]
    fn admits_empty_transcript() {
        // The bare `main` HEAD where no PRs have merged at all.
        let tmp = TempDir::new().unwrap();
        let log = write_log(tmp.path(), "");
        let report = audit_strict(&log).unwrap();
        assert!(report.all_pass);
        assert!(report.rows.is_empty());
        assert!(report.chapter_merges.is_empty());
    }

    #[test]
    fn admits_prereqs_then_chapter() {
        let mut body = String::new();
        body.push_str(&merge_line("aaaaaa1", 288, "gHashTag", "feat/phd-appendix-F-restore"));
        body.push_str(&merge_line("aaaaaa2", 289, "gHashTag", "feat/phd-lf-frontmatter"));
        body.push_str(&merge_line("aaaaaa3", 294, "gHashTag", "feat/phd-lb-springer"));
        body.push_str(&merge_line("bbbbbb1", 300, "gHashTag", "feat/phd-ch02-golden-cut"));
        body.push_str(&merge_line("bbbbbb2", 301, "gHashTag", "feat/phd-ch24-igla"));

        let report = audit_str(&body).unwrap();
        assert!(report.all_pass);
        assert_eq!(report.chapter_merges.len(), 2);
        assert_eq!(report.prereqs_seen.len(), 3);
    }

    #[test]
    fn admits_when_only_prereqs_present() {
        let mut body = String::new();
        body.push_str(&merge_line("aaaaaa1", 288, "gHashTag", "feat/phd-appendix-F-restore"));
        body.push_str(&merge_line("aaaaaa2", 289, "gHashTag", "feat/phd-lf-frontmatter"));

        let report = audit_str(&body).unwrap();
        assert!(report.all_pass);
        assert_eq!(report.prereqs_seen.len(), 2);
        assert!(report.chapter_merges.is_empty());
        assert!(report.violations.is_empty());
    }

    #[test]
    fn admits_with_unrelated_merges_interleaved() {
        let mut body = String::new();
        body.push_str(&merge_line("aaaaaa1", 1, "gHashTag", "fix/random-ci-flake"));
        body.push_str(&merge_line("aaaaaa2", 288, "gHashTag", "feat/phd-appendix-F-restore"));
        body.push_str(&merge_line("aaaaaa3", 2, "gHashTag", "chore/bump-deps"));
        body.push_str(&merge_line("aaaaaa4", 289, "gHashTag", "feat/phd-lf-frontmatter"));
        body.push_str(&merge_line("aaaaaa5", 294, "gHashTag", "feat/phd-lb-springer"));
        body.push_str(&merge_line("bbbbbb1", 300, "gHashTag", "feat/phd-ch02-golden-cut"));

        let report = audit_str(&body).unwrap();
        assert!(report.all_pass);
        assert_eq!(report.rows.len(), 6);
    }

    #[test]
    fn parse_skips_non_merge_lines() {
        // git log --first-parent emits subject lines; intermixed log
        // noise (like blank separators or summary lines without
        // " from ") must be ignored cleanly.
        let body = "\n\n   \nNot a merge subject\nanother random line\n";
        let report = audit_str(body).unwrap();
        assert!(report.rows.is_empty());
        assert!(report.all_pass);
    }

    #[test]
    fn parse_handles_trailing_punctuation_in_branch() {
        // Some merge messages end with `.`, `;`, `,`, or `)`.
        let body = "aaaaaa1 Merge pull request #294 from gHashTag/feat/phd-lb-springer.\n";
        let rows = parse_transcript(body).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].branch, "feat/phd-lb-springer");
        assert!(rows[0].is_prereq);
    }

    // ---------- falsification witnesses ------------------------------------

    #[test]
    fn falsify_git_log_read_error() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist.log");
        let err = audit_strict(&missing).unwrap_err();
        assert!(matches!(err, MergeOrderError::GitLogReadError { .. }));
        assert_eq!(err.exit_code(), 90);
    }

    #[test]
    fn falsify_malformed_merge_entry() {
        // SHA token contains non-hex characters → malformed.
        let body = "ZZZZZZZ Merge pull request #999 from gHashTag/feat/phd-ch01-golden-seed\n";
        let err = audit_str_strict(body).unwrap_err();
        match err {
            MergeOrderError::MalformedMergeEntry { line_no, .. } => assert_eq!(line_no, 1),
            other => panic!("expected MalformedMergeEntry, got {other:?}"),
        }
        assert_eq!(
            MergeOrderError::MalformedMergeEntry {
                line_no: 1,
                raw: String::new(),
            }
            .exit_code(),
            91
        );
    }

    #[test]
    fn falsify_chapter_before_prereq() {
        // Chapter merges first; prereqs land after — strict-mode violation.
        let mut body = String::new();
        body.push_str(&merge_line("bbbbbb1", 300, "gHashTag", "feat/phd-ch02-golden-cut"));
        body.push_str(&merge_line("aaaaaa1", 288, "gHashTag", "feat/phd-appendix-F-restore"));
        body.push_str(&merge_line("aaaaaa2", 289, "gHashTag", "feat/phd-lf-frontmatter"));
        body.push_str(&merge_line("aaaaaa3", 294, "gHashTag", "feat/phd-lb-springer"));

        let err = audit_str_strict(&body).unwrap_err();
        match err {
            MergeOrderError::ChapterBeforePrereq {
                ref chapter_branch,
                chapter_idx,
                missing_n,
                ref missing,
            } => {
                assert_eq!(chapter_branch, "feat/phd-ch02-golden-cut");
                assert_eq!(chapter_idx, 1);
                assert_eq!(missing_n, 3);
                assert_eq!(missing.len(), 3);
            }
            other => panic!("expected ChapterBeforePrereq, got {other:?}"),
        }
        assert_eq!(
            MergeOrderError::ChapterBeforePrereq {
                chapter_branch: "x".into(),
                chapter_idx: 1,
                missing_n: 1,
                missing: vec!["x".into()],
            }
            .exit_code(),
            92
        );
    }

    #[test]
    fn falsify_partial_prereq_before_chapter() {
        // Only two of three prereqs land before the chapter; missing #294 → strict failure.
        let mut body = String::new();
        body.push_str(&merge_line("aaaaaa1", 288, "gHashTag", "feat/phd-appendix-F-restore"));
        body.push_str(&merge_line("aaaaaa2", 289, "gHashTag", "feat/phd-lf-frontmatter"));
        body.push_str(&merge_line("bbbbbb1", 300, "gHashTag", "feat/phd-ch02-golden-cut"));
        body.push_str(&merge_line("aaaaaa3", 294, "gHashTag", "feat/phd-lb-springer"));

        let err = audit_str_strict(&body).unwrap_err();
        match err {
            MergeOrderError::ChapterBeforePrereq {
                missing_n,
                ref missing,
                ..
            } => {
                assert_eq!(missing_n, 1);
                assert_eq!(missing, &vec!["feat/phd-lb-springer".to_string()]);
            }
            other => panic!("expected ChapterBeforePrereq, got {other:?}"),
        }
    }

    #[test]
    fn falsify_prereq_never_merged() {
        // Chapter merged; only ONE prereq EVER appears → 2 prereqs never merged.
        let mut body = String::new();
        body.push_str(&merge_line("aaaaaa1", 288, "gHashTag", "feat/phd-appendix-F-restore"));
        body.push_str(&merge_line("bbbbbb1", 300, "gHashTag", "feat/phd-ch02-golden-cut"));

        let err = audit_str_strict(&body).unwrap_err();
        // The strict check raises PrereqNeverMerged before ChapterBeforePrereq
        // because PrereqNeverMerged is the deeper violation (registry bypass).
        assert!(matches!(err, MergeOrderError::PrereqNeverMerged { .. }));
        assert_eq!(err.exit_code(), 93);
    }

    // ---------- helper / anchor invariants ---------------------------------

    #[test]
    fn report_mode_never_errors_on_violations() {
        // Same transcript as `falsify_partial_prereq_before_chapter`; report mode
        // must surface the violation in the JSON without raising.
        let mut body = String::new();
        body.push_str(&merge_line("aaaaaa1", 288, "gHashTag", "feat/phd-appendix-F-restore"));
        body.push_str(&merge_line("bbbbbb1", 300, "gHashTag", "feat/phd-ch02-golden-cut"));

        let report = audit_str(&body).unwrap();
        assert!(!report.all_pass);
        // chapter merge present but only 1 of 3 prereqs seen
        assert_eq!(report.chapter_merges.len(), 1);
        assert_eq!(report.prereqs_seen.len(), 1);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].missing.len(), 2);
    }

    #[test]
    fn render_report_json_round_trips() {
        let body = merge_line("aaaaaa1", 288, "gHashTag", "feat/phd-appendix-F-restore");
        let report = audit_str(&body).unwrap();
        let json = render_report_json(&report).unwrap();
        let back: OrderReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, back);
    }

    #[test]
    fn phase_a_prereqs_match_one_shot_v2_section_3() {
        // The pre-registered list is mirrored verbatim in
        // assertions/witness/merge_order_gate.toml; bumping it requires
        // a sibling commit per R10.
        assert_eq!(PHASE_A_PREREQS.len(), 3);
        assert!(PHASE_A_PREREQS.contains(&"feat/phd-appendix-F-restore"));
        assert!(PHASE_A_PREREQS.contains(&"feat/phd-lf-frontmatter"));
        assert!(PHASE_A_PREREQS.contains(&"feat/phd-lb-springer"));
        assert_eq!(CHAPTER_PREFIX, "feat/phd-ch");
    }
}
