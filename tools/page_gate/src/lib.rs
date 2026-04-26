//! LT page-count gate — Phase D witness library.
//!
//! Pure structural witness for the «Flos Aureus» PhD monograph (trios#265):
//! validates that a tectonic-built PDF has a page count inside the
//! pre-registered band declared by the ONE SHOT v2.0 §3 Phase D.
//!
//! ## Pre-registered band
//!
//! `MIN_PAGES = 250`, `MAX_PAGES = 350`, both inclusive.
//!
//! These literals are mirrored verbatim in `assertions/witness/page_gate.toml`
//! as the immutable witness manifest; any future change must bump the manifest
//! and ship a sibling commit.
//!
//! ## R1 / R6 / R10 compliance
//!
//! - R1: pure Rust, no `.py` / `.sh` runtime.
//! - R6: greenfield `tools/page_gate/` — owns its files exclusively.
//! - R10: this crate ships in one atomic commit per the ONE SHOT contract.
//!
//! ## R8 falsification witnesses
//!
//! Each rejection variant of [`PageGateError`] corresponds to a single
//! failure mode the witness script must surface; matched by the
//! `falsify_*` unit tests in [`crate::tests`].
//!
//! ## L-R14 traceability
//!
//! The two band literals are constants here, mirrored in the witness manifest.
//! The `--print-anchors` mode of the binary dumps them as JSON for audit.
//!
//! Refs: ONE SHOT v2.0 §3 Phase D · §5 witness table · trios#265.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::path::Path;

use thiserror::Error;

// ---------------------------------------------------------------------------
// Pre-registered band (Phase D §3 / §5 of ONE SHOT v2.0)
// ---------------------------------------------------------------------------

/// Minimum admissible page count — pre-registered in ONE SHOT v2.0 §3 Phase D.
pub const MIN_PAGES: u32 = 250;

/// Maximum admissible page count — pre-registered in ONE SHOT v2.0 §3 Phase D.
pub const MAX_PAGES: u32 = 350;

// Compile-time sanity: band non-degenerate.
const _: () = assert!(MIN_PAGES < MAX_PAGES, "page band degenerate");
const _: () = assert!(MIN_PAGES > 0, "MIN_PAGES must be positive");

// ---------------------------------------------------------------------------
// Error model
// ---------------------------------------------------------------------------

/// Reasons a PDF is rejected by the gate.
///
/// Each variant carries enough context to reconstruct the falsification
/// witness without re-running the binary.  Disjoint exit codes 60..=63
/// keep LT's namespace clear of L7 (0..=10), L15 (21..=30), and L-h4
/// (50..=53).
#[derive(Debug, Error)]
pub enum PageGateError {
    /// PDF contains fewer than [`MIN_PAGES`] pages.
    #[error("LT page-gate: {pages} < MIN_PAGES ({min})")]
    BelowBand {
        /// Observed page count.
        pages: u32,
        /// Pre-registered floor.
        min: u32,
    },

    /// PDF contains more than [`MAX_PAGES`] pages.
    #[error("LT page-gate: {pages} > MAX_PAGES ({max})")]
    AboveBand {
        /// Observed page count.
        pages: u32,
        /// Pre-registered ceiling.
        max: u32,
    },

    /// I/O error opening the PDF (missing, unreadable, permission denied).
    #[error("LT page-gate: cannot open PDF at {path:?}: {detail}")]
    Io {
        /// Path the binary tried to open.
        path: String,
        /// Underlying I/O error string.
        detail: String,
    },

    /// PDF parsed but page-tree is malformed or empty.
    #[error("LT page-gate: malformed PDF {path:?}: {reason}")]
    Malformed {
        /// Path that failed to parse.
        path: String,
        /// Human-readable reason from the PDF parser.
        reason: String,
    },
}

impl PageGateError {
    /// Disjoint exit code for shell tooling and CI.  60..=63 is LT's reserved
    /// namespace per the lane-disjointness convention (L7=0..=10, L15=21..=30,
    /// L-h4=50..=53, LT=60..=63).
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::BelowBand { .. } => 60,
            Self::AboveBand { .. } => 61,
            Self::Io { .. } => 62,
            Self::Malformed { .. } => 63,
        }
    }
}

// ---------------------------------------------------------------------------
// Page-count extraction
// ---------------------------------------------------------------------------

/// Count pages in `pdf_path` using `lopdf`'s page-tree walker.
///
/// Pure / total: returns [`PageGateError::Io`] for unreadable paths and
/// [`PageGateError::Malformed`] for parse / page-tree errors; never panics.
pub fn count_pages<P: AsRef<Path>>(pdf_path: P) -> Result<u32, PageGateError> {
    let path = pdf_path.as_ref();
    let path_str = path.display().to_string();

    let doc = lopdf::Document::load(path).map_err(|e| {
        // lopdf returns a single Error enum; classify I/O vs malformed by
        // peeking at the message rather than matching unstable variants.
        let msg = e.to_string();
        if msg.contains("No such file")
            || msg.contains("os error")
            || msg.contains("Permission denied")
        {
            PageGateError::Io {
                path: path_str.clone(),
                detail: msg,
            }
        } else {
            PageGateError::Malformed {
                path: path_str.clone(),
                reason: msg,
            }
        }
    })?;

    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err(PageGateError::Malformed {
            path: path_str,
            reason: "page tree resolved to zero pages".to_owned(),
        });
    }

    // get_pages() returns BTreeMap<u32, ObjectId>; len() is the page count.
    let n = u32::try_from(pages.len()).map_err(|_| PageGateError::Malformed {
        path: path_str,
        reason: format!("page count {} exceeds u32::MAX", pages.len()),
    })?;
    Ok(n)
}

/// Pure admissibility predicate over a known page count.
///
/// Total / panic-free for any `u32`.
pub fn admit(pages: u32) -> Result<(), PageGateError> {
    if pages < MIN_PAGES {
        return Err(PageGateError::BelowBand {
            pages,
            min: MIN_PAGES,
        });
    }
    if pages > MAX_PAGES {
        return Err(PageGateError::AboveBand {
            pages,
            max: MAX_PAGES,
        });
    }
    Ok(())
}

/// One-shot helper: count + admit.
pub fn check_pdf<P: AsRef<Path>>(pdf_path: P) -> Result<u32, PageGateError> {
    let pages = count_pages(pdf_path)?;
    admit(pages)?;
    Ok(pages)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Pure admit() — no PDF I/O required ----------

    #[test]
    fn admit_in_band() {
        assert!(admit(MIN_PAGES).is_ok());
        assert!(admit(MAX_PAGES).is_ok());
        assert!(admit((MIN_PAGES + MAX_PAGES) / 2).is_ok());
    }

    #[test]
    fn falsify_below_band() {
        let r = admit(MIN_PAGES - 1);
        match r {
            Err(PageGateError::BelowBand { pages, min }) => {
                assert_eq!(pages, MIN_PAGES - 1);
                assert_eq!(min, MIN_PAGES);
            }
            other => panic!("expected BelowBand, got {other:?}"),
        }
    }

    #[test]
    fn falsify_above_band() {
        let r = admit(MAX_PAGES + 1);
        match r {
            Err(PageGateError::AboveBand { pages, max }) => {
                assert_eq!(pages, MAX_PAGES + 1);
                assert_eq!(max, MAX_PAGES);
            }
            other => panic!("expected AboveBand, got {other:?}"),
        }
    }

    #[test]
    fn falsify_zero_pages() {
        // 0 < MIN_PAGES, so BelowBand.
        assert!(matches!(admit(0), Err(PageGateError::BelowBand { .. })));
    }

    #[test]
    fn admit_is_total_for_all_u32() {
        // Spot-check: any u32 returns a Result, never panics.
        for p in [0_u32, 1, 100, MIN_PAGES, MAX_PAGES, u32::MAX] {
            let _ = admit(p);
        }
    }

    // ---------- I/O error mapping ----------

    #[test]
    fn count_pages_missing_file() {
        let r = count_pages("/no/such/path/this-cannot-exist-9c5d.pdf");
        match r {
            Err(PageGateError::Io { .. }) => {}
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn count_pages_malformed_pdf() {
        // Write garbage bytes to a temp file.
        let dir = std::env::temp_dir();
        let p = dir.join(format!("page_gate_garbage_{}.pdf", std::process::id()));
        std::fs::write(&p, b"this is definitely not a pdf").unwrap();
        let r = count_pages(&p);
        let _ = std::fs::remove_file(&p);
        assert!(
            matches!(r, Err(PageGateError::Malformed { .. } | PageGateError::Io { .. })),
            "expected Malformed/Io, got {r:?}"
        );
    }

    // ---------- Real PDF round-trip (skipped if PDF absent) ----------

    #[test]
    fn count_pages_on_repo_pdf_if_present() {
        // tests run from crate dir (tools/page_gate/) — repo PDF is two up.
        let candidates = [
            "../../docs/phd/main.pdf",
            "../docs/phd/main.pdf",
            "docs/phd/main.pdf",
        ];
        let pdf = candidates.iter().find(|p| std::path::Path::new(p).exists());
        let Some(pdf) = pdf else {
            eprintln!("[skip] repo main.pdf not found at any candidate path");
            return;
        };
        let n = count_pages(pdf).expect("should parse repo pdf");
        // Sanity bounds — not the gate band, just structural.
        assert!(n >= 1, "page count should be >= 1");
        assert!(n < 10_000, "page count should be sane");
        eprintln!("[ok] repo main.pdf has {n} pages");
    }

    // ---------- Exit-code disjointness ----------

    #[test]
    fn exit_codes_distinct_and_in_lt_range() {
        let codes = [
            PageGateError::BelowBand { pages: 0, min: 250 }.exit_code(),
            PageGateError::AboveBand {
                pages: 1000,
                max: 350,
            }
            .exit_code(),
            PageGateError::Io {
                path: "x".into(),
                detail: "y".into(),
            }
            .exit_code(),
            PageGateError::Malformed {
                path: "x".into(),
                reason: "y".into(),
            }
            .exit_code(),
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i], codes[j], "exit codes must be disjoint");
            }
        }
        for c in codes {
            assert!(
                (60..=63).contains(&c),
                "exit {c} outside LT reserved 60..=63"
            );
        }
    }

    // ---------- L-R14 anchor canary ----------

    #[test]
    fn band_constants_match_one_shot_v2_phase_d() {
        // Pre-registered values per ONE SHOT v2.0 §3 Phase D.
        assert_eq!(MIN_PAGES, 250, "MIN_PAGES anchor drift");
        assert_eq!(MAX_PAGES, 350, "MAX_PAGES anchor drift");
    }
}
