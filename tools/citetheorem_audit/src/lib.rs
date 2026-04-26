//! `citetheorem_audit` — R14 retrofit witness library.
//!
//! Pure structural witness for the «Flos Aureus» PhD monograph (trios#265):
//! audits LaTeX chapter files for `\citetheorem{INV-N}` coverage against
//! the canonical theorem registry in `assertions/igla_assertions.json`.
//!
//! ## R14 contract
//!
//! ONE SHOT v2.0 §5 (falsification table): *every chapter must
//! `\citetheorem{INV-N}` against the Coq citation map*.  This crate
//! mechanises that rule: each `*.tex` file under `--chapters-dir` MUST
//! contain at least [`MIN_CITETHEOREM_PER_CHAPTER`] `\citetheorem{...}`
//! calls, and every cited token MUST resolve to an `INV-N` entry in
//! `assertions/igla_assertions.json`.
//!
//! ## R5 honesty
//!
//! The auditor never edits a `.tex` file or `igla_assertions.json` —
//! it merely reports the gap.  Chapter retrofit work is owned by the
//! `phd-chapter-author` lanes (`feat/phd-ch{15,23,30}-citetheorem` …).
//!
//! On `main` at the time this binary lands, retrofit branches have not
//! merged yet; the strict-mode witness will therefore EXIT with code
//! 83 ([`CiteAuditError::BelowMinCitations`]).  That is the *expected*
//! pre-merge verdict and is documented in the DONE comment.
//!
//! ## R1 / R6 / R10 compliance
//!
//! - R1: pure Rust.  No `.py` / `.sh` runtime.
//! - R6: greenfield `tools/citetheorem_audit/` — owns its files exclusively.
//! - R10: this crate ships in one atomic commit per the ONE SHOT contract.
//!
//! ## R8 falsification witnesses
//!
//! Each rejection variant of [`CiteAuditError`] is paired with a
//! `falsify_*` unit test that constructs the exact failure mode and
//! asserts the corresponding exit code.
//!
//! ## L-R14 traceability
//!
//! The minimum-citation literal is a constant here, mirrored verbatim
//! in `assertions/witness/citetheorem_audit.toml`; the binary's
//! `--print-anchors` mode dumps all anchors as JSON for audit.
//!
//! Refs: ONE SHOT v2.0 §3 Phase D · §5 witness table · trios#265.
//! Trinity anchor: `phi^2 + phi^-2 = 3` · DOI 10.5281/zenodo.19227877.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Pre-registered constants (Phase D §5 of ONE SHOT v2.0)
// ---------------------------------------------------------------------------

/// Minimum admissible `\citetheorem{INV-N}` calls per chapter file.
///
/// Pre-registered in ONE SHOT v2.0 §5 (R14 retrofit witness row).  Mirrored
/// verbatim in `assertions/witness/citetheorem_audit.toml` and dumped by the
/// binary's `--print-anchors` mode.
pub const MIN_CITETHEOREM_PER_CHAPTER: u32 = 1;

/// File-name suffix selected by the chapter scanner.
pub const CHAPTER_SUFFIX: &str = ".tex";

// Compile-time sanity: minimum non-zero (a zero floor would make the gate
// a no-op and break R8).
const _: () = assert!(
    MIN_CITETHEOREM_PER_CHAPTER >= 1,
    "MIN_CITETHEOREM_PER_CHAPTER must be >= 1 (R8: gate must reject zero coverage)"
);

// ---------------------------------------------------------------------------
// Error model — exit codes 80..=83 (disjoint per coq-runtime-invariants v1.1)
// ---------------------------------------------------------------------------

/// Reasons the citetheorem audit refuses to admit the monograph state.
///
/// Exit codes 80..=83 are disjoint from L7 (0..=10), L15 (21..=30),
/// L-h4 (50..=53), LT (60..=63), and LA (70..=74).
#[derive(Debug, Error)]
pub enum CiteAuditError {
    /// `assertions/igla_assertions.json` could not be read or parsed.
    #[error("citetheorem-audit: assertions load failure: {detail}")]
    AssertionsLoadError {
        /// Human-readable cause (formatted IO/serde error).
        detail: String,
    },

    /// Chapters directory could not be enumerated.
    #[error("citetheorem-audit: chapters dir unreadable ({path}): {detail}")]
    ChaptersDirError {
        /// Path that failed to enumerate.
        path: PathBuf,
        /// Human-readable cause (formatted IO error).
        detail: String,
    },

    /// A `\citetheorem{INV-N}` call references an `INV-N` not present in
    /// the assertions registry.
    #[error("citetheorem-audit: chapter {chapter} cites unknown {token:?}")]
    UnknownInvCited {
        /// Chapter file name (relative to `--chapters-dir`).
        chapter: String,
        /// The exact token written between the braces of `\citetheorem{...}`.
        token: String,
    },

    /// At least one chapter file has fewer than [`MIN_CITETHEOREM_PER_CHAPTER`]
    /// `\citetheorem{...}` calls.
    #[error(
        "citetheorem-audit: {n_below} chapter(s) below min={min}: {chapters:?}"
    )]
    BelowMinCitations {
        /// Number of chapters short of the floor.
        n_below: usize,
        /// Pre-registered floor.
        min: u32,
        /// File-names of the offending chapters (sorted).
        chapters: Vec<String>,
    },
}

impl CiteAuditError {
    /// Stable exit-code mapping (disjoint from sibling lanes).
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::AssertionsLoadError { .. } => 80,
            Self::ChaptersDirError { .. } => 81,
            Self::UnknownInvCited { .. } => 82,
            Self::BelowMinCitations { .. } => 83,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Per-chapter audit row used in [`AuditReport`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChapterRow {
    /// File name (relative to `--chapters-dir`).
    pub chapter: String,
    /// Number of `\citetheorem{...}` calls in the file.
    pub citations: u32,
    /// Distinct `INV-N` tokens cited (sorted).
    pub invs_cited: Vec<String>,
    /// `true` iff `citations >= MIN_CITETHEOREM_PER_CHAPTER`.
    pub passes_floor: bool,
}

/// Report-mode output (`--report` flag on the binary).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditReport {
    /// Per-chapter rows (sorted by file name).
    pub rows: Vec<ChapterRow>,
    /// `INV-N` tokens recognised in the assertions registry.
    pub known_invs: Vec<String>,
    /// `\citetheorem{...}` tokens that did NOT resolve to a known `INV-N`.
    pub unknown_tokens: Vec<UnknownToken>,
    /// `true` iff every chapter passes the floor AND no unknown tokens exist.
    pub all_pass: bool,
    /// Pre-registered floor, mirrored from [`MIN_CITETHEOREM_PER_CHAPTER`].
    pub min_citetheorem_per_chapter: u32,
}

/// One unresolved `\citetheorem{...}` reference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnknownToken {
    /// Chapter file name where the token appeared.
    pub chapter: String,
    /// Verbatim contents between the braces of `\citetheorem{...}`.
    pub token: String,
}

/// Strict-mode entry point.
///
/// Returns `Ok(report)` iff every chapter file under `chapters_dir`
/// has `>= MIN_CITETHEOREM_PER_CHAPTER` `\citetheorem{INV-N}` calls and
/// every cited token resolves to an `INV-N` entry in
/// `assertions/igla_assertions.json`.
pub fn audit_strict(
    chapters_dir: &Path,
    assertions_path: &Path,
) -> Result<AuditReport, CiteAuditError> {
    let report = audit_report(chapters_dir, assertions_path)?;

    if let Some(token) = report.unknown_tokens.first() {
        return Err(CiteAuditError::UnknownInvCited {
            chapter: token.chapter.clone(),
            token: token.token.clone(),
        });
    }

    let below: Vec<String> = report
        .rows
        .iter()
        .filter(|r| !r.passes_floor)
        .map(|r| r.chapter.clone())
        .collect();
    if !below.is_empty() {
        return Err(CiteAuditError::BelowMinCitations {
            n_below: below.len(),
            min: MIN_CITETHEOREM_PER_CHAPTER,
            chapters: below,
        });
    }

    Ok(report)
}

/// Report-mode entry point: never errors on R14 violations, only on
/// I/O / parse failures.  Suitable for `--report` audit-only invocations.
pub fn audit_report(
    chapters_dir: &Path,
    assertions_path: &Path,
) -> Result<AuditReport, CiteAuditError> {
    let known = load_known_invs(assertions_path)?;
    let mut rows = scan_chapters(chapters_dir)?;
    rows.sort_by(|a, b| a.chapter.cmp(&b.chapter));

    // Resolve unknowns against the registry, dedup deterministically.
    let mut unknown_tokens: Vec<UnknownToken> = Vec::new();
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for row in &rows {
        for tok in &row.invs_cited {
            if !known.contains(tok) {
                let key = (row.chapter.clone(), tok.clone());
                if seen.insert(key.clone()) {
                    unknown_tokens.push(UnknownToken {
                        chapter: key.0,
                        token: key.1,
                    });
                }
            }
        }
    }

    let all_pass = unknown_tokens.is_empty()
        && rows.iter().all(|r| r.passes_floor);

    Ok(AuditReport {
        rows,
        known_invs: known.into_iter().collect(),
        unknown_tokens,
        all_pass,
        min_citetheorem_per_chapter: MIN_CITETHEOREM_PER_CHAPTER,
    })
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Parse the `INV-N` ids declared in `assertions/igla_assertions.json`.
///
/// We deliberately do NOT depend on the full schema — we walk every
/// JSON value looking for the exact key sequence `"id": "INV-..."`.
/// This keeps the auditor stable across schema bumps (R6: we never
/// touch the assertions file) and obeys R5 (we honestly report what
/// the JSON declares, not what we wish it declared).
pub(crate) fn load_known_invs(path: &Path) -> Result<BTreeSet<String>, CiteAuditError> {
    let bytes = fs::read(path).map_err(|e| CiteAuditError::AssertionsLoadError {
        detail: format!("read {}: {e}", path.display()),
    })?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| CiteAuditError::AssertionsLoadError {
            detail: format!("parse {}: {e}", path.display()),
        })?;

    let mut out = BTreeSet::new();
    collect_inv_ids(&value, &mut out);
    Ok(out)
}

fn collect_inv_ids(v: &serde_json::Value, out: &mut BTreeSet<String>) {
    match v {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(s)) = map.get("id") {
                if is_inv_token(s) {
                    out.insert(s.clone());
                }
            }
            for (_, child) in map {
                collect_inv_ids(child, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for child in arr {
                collect_inv_ids(child, out);
            }
        }
        _ => {}
    }
}

/// Scan a chapters directory for `*.tex` files and extract their
/// `\citetheorem{...}` payloads.
fn scan_chapters(dir: &Path) -> Result<Vec<ChapterRow>, CiteAuditError> {
    let entries = fs::read_dir(dir).map_err(|e| CiteAuditError::ChaptersDirError {
        path: dir.to_path_buf(),
        detail: e.to_string(),
    })?;

    let mut rows = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| CiteAuditError::ChaptersDirError {
            path: dir.to_path_buf(),
            detail: e.to_string(),
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.ends_with(CHAPTER_SUFFIX) => n.to_string(),
            _ => continue,
        };

        let body = fs::read_to_string(&path).map_err(|e| CiteAuditError::ChaptersDirError {
            path: path.clone(),
            detail: e.to_string(),
        })?;

        let tokens = extract_citetheorem_tokens(&body);
        let citations = tokens.len() as u32;
        let mut distinct: BTreeMap<String, ()> = BTreeMap::new();
        for t in tokens {
            distinct.insert(t, ());
        }
        let invs_cited: Vec<String> = distinct.into_keys().collect();

        rows.push(ChapterRow {
            chapter: name,
            citations,
            invs_cited,
            passes_floor: citations >= MIN_CITETHEOREM_PER_CHAPTER,
        });
    }

    Ok(rows)
}

/// Extract every payload between `\citetheorem{` and the matching closing
/// brace.  Multiple comma-separated arguments inside one pair of braces
/// (`\citetheorem{INV-2,INV-3}`) are split into individual tokens.
///
/// Whitespace around tokens is trimmed.  Lines beginning with a `%`
/// comment are skipped (LaTeX line comments).  Backslash-escaped `%`
/// (`\%`) is kept inline.
pub(crate) fn extract_citetheorem_tokens(body: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for raw_line in body.lines() {
        let line = strip_latex_line_comment(raw_line);
        let bytes = line.as_bytes();
        let needle = b"\\citetheorem{";
        let mut i = 0;
        while i + needle.len() <= bytes.len() {
            if &bytes[i..i + needle.len()] == needle {
                let start = i + needle.len();
                if let Some(rel_end) = bytes[start..].iter().position(|b| *b == b'}') {
                    let payload = &line[start..start + rel_end];
                    for raw in payload.split(',') {
                        let tok = raw.trim();
                        if !tok.is_empty() {
                            tokens.push(tok.to_string());
                        }
                    }
                    i = start + rel_end + 1;
                    continue;
                }
                // Unterminated `\citetheorem{` — abandon this line.
                break;
            }
            i += 1;
        }
    }
    tokens
}

/// Strip the part of a LaTeX line after an *unescaped* `%`.
fn strip_latex_line_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && (i == 0 || bytes[i - 1] != b'\\') {
            return &line[..i];
        }
        i += 1;
    }
    line
}

/// `INV-` prefix + at least one ASCII digit.
pub(crate) fn is_inv_token(s: &str) -> bool {
    let rest = match s.strip_prefix("INV-") {
        Some(r) => r,
        None => return false,
    };
    !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
}

/// Convenience helper for downstream tools (e.g. CI summary scripts):
/// load the report and convert it to canonical pretty-printed JSON.
pub fn render_report_json(report: &AuditReport) -> Result<String, io::Error> {
    serde_json::to_string_pretty(report).map_err(io::Error::other)
}

// ---------------------------------------------------------------------------
// Tests — falsification witnesses (one per CiteAuditError variant) plus
// happy-path coverage.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write as fwrite;
    use tempfile::TempDir;

    fn write_assertions(dir: &Path) -> PathBuf {
        let p = dir.join("igla_assertions.json");
        let body = serde_json::json!({
            "_metadata": { "schema_version": "1.0.0" },
            "invariants": [
                { "id": "INV-1",  "name": "BPB" },
                { "id": "INV-2",  "name": "ASHA" },
                { "id": "INV-3",  "name": "GF16" },
                { "id": "INV-7",  "name": "Victory" },
                { "id": "INV-13", "name": "QK gain" }
            ]
        });
        fwrite(&p, body.to_string().as_bytes()).unwrap();
        p
    }

    fn write_chapter(dir: &Path, name: &str, body: &str) {
        let p = dir.join(name);
        fwrite(p, body.as_bytes()).unwrap();
    }

    // ---------- happy path -------------------------------------------------

    #[test]
    fn admits_chapter_with_one_known_citetheorem() {
        let tmp = TempDir::new().unwrap();
        let assertions = write_assertions(tmp.path());
        let chapters = tmp.path().join("chapters");
        fs::create_dir(&chapters).unwrap();
        write_chapter(
            &chapters,
            "24-igla-architecture.tex",
            "Body text \\citetheorem{INV-13} more text.\n",
        );

        let report = audit_strict(&chapters, &assertions).unwrap();
        assert!(report.all_pass);
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].citations, 1);
        assert_eq!(report.rows[0].invs_cited, vec!["INV-13".to_string()]);
        assert_eq!(report.min_citetheorem_per_chapter, MIN_CITETHEOREM_PER_CHAPTER);
    }

    #[test]
    fn admits_multi_token_brace() {
        let tmp = TempDir::new().unwrap();
        let assertions = write_assertions(tmp.path());
        let chapters = tmp.path().join("chapters");
        fs::create_dir(&chapters).unwrap();
        write_chapter(
            &chapters,
            "27-trinity-identity.tex",
            "By \\citetheorem{INV-2, INV-3, INV-7}, the bound holds.\n",
        );

        let report = audit_strict(&chapters, &assertions).unwrap();
        assert!(report.all_pass);
        let row = &report.rows[0];
        assert_eq!(row.citations, 3);
        assert_eq!(
            row.invs_cited,
            vec![
                "INV-2".to_string(),
                "INV-3".to_string(),
                "INV-7".to_string()
            ]
        );
    }

    #[test]
    fn report_mode_returns_unknowns_without_failing() {
        let tmp = TempDir::new().unwrap();
        let assertions = write_assertions(tmp.path());
        let chapters = tmp.path().join("chapters");
        fs::create_dir(&chapters).unwrap();
        write_chapter(
            &chapters,
            "01-golden-egg.tex",
            "stub \\citetheorem{INV-99}\n",
        );
        write_chapter(&chapters, "02-golden-cut.tex", "no citation here\n");

        let report = audit_report(&chapters, &assertions).expect("report mode never errors on R14");
        assert!(!report.all_pass);
        assert_eq!(report.unknown_tokens.len(), 1);
        assert_eq!(report.unknown_tokens[0].token, "INV-99");
        // 02 has zero citations, fails the floor.
        let r02 = report.rows.iter().find(|r| r.chapter.starts_with("02")).unwrap();
        assert!(!r02.passes_floor);
    }

    #[test]
    fn ignores_non_tex_files_and_subdirs() {
        let tmp = TempDir::new().unwrap();
        let assertions = write_assertions(tmp.path());
        let chapters = tmp.path().join("chapters");
        fs::create_dir(&chapters).unwrap();
        write_chapter(&chapters, "README.md", "\\citetheorem{INV-1}\n");
        fs::create_dir(chapters.join("backup")).unwrap();
        write_chapter(
            &chapters,
            "00-monad.tex",
            "\\citetheorem{INV-1}\n",
        );

        let report = audit_strict(&chapters, &assertions).unwrap();
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].chapter, "00-monad.tex");
    }

    #[test]
    fn skips_latex_line_comments() {
        let tmp = TempDir::new().unwrap();
        let assertions = write_assertions(tmp.path());
        let chapters = tmp.path().join("chapters");
        fs::create_dir(&chapters).unwrap();
        write_chapter(
            &chapters,
            "10-golden-bloom.tex",
            "% \\citetheorem{INV-99} commented out\nbody \\citetheorem{INV-1} live.\n",
        );

        let report = audit_strict(&chapters, &assertions).unwrap();
        let row = &report.rows[0];
        assert_eq!(row.citations, 1);
        assert_eq!(row.invs_cited, vec!["INV-1".to_string()]);
    }

    #[test]
    fn keeps_escaped_percent_inline() {
        // `\%` in LaTeX is a literal percent and must NOT terminate parsing.
        let body = "p \\% rate then \\citetheorem{INV-3}\n";
        let tokens = extract_citetheorem_tokens(body);
        assert_eq!(tokens, vec!["INV-3".to_string()]);
    }

    #[test]
    fn dedups_distinct_invs_per_chapter() {
        let body = "\\citetheorem{INV-7}\\citetheorem{INV-7}\\citetheorem{INV-2}";
        let tokens = extract_citetheorem_tokens(body);
        assert_eq!(tokens.len(), 3, "raw token count is preserved");

        let tmp = TempDir::new().unwrap();
        let assertions = write_assertions(tmp.path());
        let chapters = tmp.path().join("chapters");
        fs::create_dir(&chapters).unwrap();
        write_chapter(&chapters, "07-golden-sprout.tex", body);

        let report = audit_strict(&chapters, &assertions).unwrap();
        let row = &report.rows[0];
        assert_eq!(row.citations, 3);
        assert_eq!(
            row.invs_cited,
            vec!["INV-2".to_string(), "INV-7".to_string()]
        );
    }

    // ---------- falsification witnesses ------------------------------------

    #[test]
    fn falsify_assertions_load_error() {
        let tmp = TempDir::new().unwrap();
        let chapters = tmp.path().join("chapters");
        fs::create_dir(&chapters).unwrap();
        let missing = tmp.path().join("missing.json");

        let err = audit_strict(&chapters, &missing).unwrap_err();
        assert!(matches!(err, CiteAuditError::AssertionsLoadError { .. }));
        assert_eq!(err.exit_code(), 80);
    }

    #[test]
    fn falsify_assertions_load_error_on_bad_json() {
        let tmp = TempDir::new().unwrap();
        let chapters = tmp.path().join("chapters");
        fs::create_dir(&chapters).unwrap();
        let bad = tmp.path().join("bad.json");
        fs::write(&bad, b"{ not json").unwrap();

        let err = audit_strict(&chapters, &bad).unwrap_err();
        assert!(matches!(err, CiteAuditError::AssertionsLoadError { .. }));
        assert_eq!(err.exit_code(), 80);
    }

    #[test]
    fn falsify_chapters_dir_error() {
        let tmp = TempDir::new().unwrap();
        let assertions = write_assertions(tmp.path());
        let missing = tmp.path().join("does-not-exist");

        let err = audit_strict(&missing, &assertions).unwrap_err();
        assert!(matches!(err, CiteAuditError::ChaptersDirError { .. }));
        assert_eq!(err.exit_code(), 81);
    }

    #[test]
    fn falsify_unknown_inv_cited() {
        let tmp = TempDir::new().unwrap();
        let assertions = write_assertions(tmp.path());
        let chapters = tmp.path().join("chapters");
        fs::create_dir(&chapters).unwrap();
        write_chapter(
            &chapters,
            "15-kepler-solids.tex",
            "Body \\citetheorem{INV-999}.\n",
        );

        let err = audit_strict(&chapters, &assertions).unwrap_err();
        match err {
            CiteAuditError::UnknownInvCited { ref chapter, ref token } => {
                assert_eq!(chapter, "15-kepler-solids.tex");
                assert_eq!(token, "INV-999");
            }
            other => panic!("expected UnknownInvCited, got {other:?}"),
        }
        assert_eq!(CiteAuditError::UnknownInvCited {
            chapter: "x".into(),
            token: "y".into(),
        }
        .exit_code(), 82);
    }

    #[test]
    fn falsify_below_min_citations() {
        let tmp = TempDir::new().unwrap();
        let assertions = write_assertions(tmp.path());
        let chapters = tmp.path().join("chapters");
        fs::create_dir(&chapters).unwrap();
        // Three empty chapters — exactly the pre-merge state on `main`
        // for chapter retrofit branches `feat/phd-ch{15,23,30}-citetheorem`.
        write_chapter(&chapters, "15-kepler-solids.tex", "no theorem here\n");
        write_chapter(&chapters, "23-gf16-algebra.tex", "or here\n");
        write_chapter(&chapters, "30-golden-imagery.tex", "or here either\n");

        let err = audit_strict(&chapters, &assertions).unwrap_err();
        match err {
            CiteAuditError::BelowMinCitations { n_below, min, ref chapters } => {
                assert_eq!(n_below, 3);
                assert_eq!(min, MIN_CITETHEOREM_PER_CHAPTER);
                assert_eq!(
                    chapters,
                    &vec![
                        "15-kepler-solids.tex".to_string(),
                        "23-gf16-algebra.tex".to_string(),
                        "30-golden-imagery.tex".to_string(),
                    ]
                );
            }
            other => panic!("expected BelowMinCitations, got {other:?}"),
        }
        assert_eq!(
            CiteAuditError::BelowMinCitations {
                n_below: 1,
                min: MIN_CITETHEOREM_PER_CHAPTER,
                chapters: vec!["x".into()],
            }
            .exit_code(),
            83
        );
    }

    // ---------- L-R14 anchor & helper invariants ---------------------------

    #[test]
    fn is_inv_token_recognises_canonical_ids() {
        for ok in ["INV-1", "INV-2", "INV-7", "INV-13", "INV-99"] {
            assert!(is_inv_token(ok), "expected {ok} to be an INV token");
        }
        for bad in ["INV", "INV-", "inv-1", "INV-1a", "INV-X"] {
            assert!(!is_inv_token(bad), "expected {bad} to be rejected");
        }
    }

    #[test]
    fn min_citetheorem_constant_is_pre_registered() {
        // The literal lives in tools/citetheorem_audit/src/lib.rs and is
        // mirrored in assertions/witness/citetheorem_audit.toml.  Bumping
        // it requires a sibling commit per R10.
        assert_eq!(MIN_CITETHEOREM_PER_CHAPTER, 1);
    }

    #[test]
    fn render_report_json_round_trips() {
        let tmp = TempDir::new().unwrap();
        let assertions = write_assertions(tmp.path());
        let chapters = tmp.path().join("chapters");
        fs::create_dir(&chapters).unwrap();
        write_chapter(
            &chapters,
            "00-monad.tex",
            "\\citetheorem{INV-1}\n",
        );

        let report = audit_report(&chapters, &assertions).unwrap();
        let json = render_report_json(&report).unwrap();
        let back: AuditReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, back);
    }
}
