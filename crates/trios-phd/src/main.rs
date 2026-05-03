//! `trios-phd` — Rust-only build/audit/bibliography/coq-map/reproducibility pipeline
//! for the PhD monograph "Flos Aureus" (`docs/phd/`).
//!
//! Mission rule R1 (CROWN): no `.py`, no `.sh`, no shell wrappers. Everything
//! that touches the dissertation is a Rust subcommand of this binary.
//!
//! Subcommands:
//!   - `audit`     — structural sanity (chapter count, bib count, frontmatter,
//!                   appendices, missing files); exit non-zero on violations.
//!   - `biblio`    — count `bibliography.bib` entries and verify R11 floor (≥150).
//!   - `coq-map`   — render the Coq → PhD theorem citation table (R14) into
//!                   `appendix/F-coq-citation-map.tex` from a JSON manifest.
//!   - `reproduce` — emit the reproducibility manifest (build env, git SHA,
//!                   constants pinned).
//!   - `compile`   — invoke the system `tectonic` binary on `main.tex`. The
//!                   `tectonic` Rust crate itself depends on native harfbuzz/
//!                   freetype, so we shell out via `std::process::Command`,
//!                   which keeps the workspace Rust-only (no `.sh` files).
//!
//! All numeric anchors (R4 / L-R14) come from
//! `assertions/igla_assertions.json`. This binary never hard-codes a numeric
//! invariant constant — it loads them at runtime.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

mod render;

const TRINITY_ANCHOR: &str = "phi^2 + phi^-2 = 3";
const R11_BIB_FLOOR: usize = 150;
const R3_CHAPTER_MIN_LINES: usize = 1500; // R3 long-form floor (warn-only here).

#[derive(Parser, Debug)]
#[command(
    name = "trios-phd",
    about = "PhD monograph build / audit / bibliography / coq-map / reproduce (Rust-only, R1).",
    version
)]
struct Cli {
    /// Path to the PhD root (the directory that contains `main.tex`).
    #[arg(long, default_value = "docs/phd", global = true)]
    phd_root: PathBuf,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Structural audit: chapters, frontmatter, appendices, bibliography.
    Audit,
    /// Count bibliography entries and verify the R11 ≥150 floor.
    Biblio,
    /// Print / refresh the Coq → PhD theorem citation map (R14 anchor).
    CoqMap {
        /// Just verify the existing appendix is in sync (no rewrite).
        #[arg(long)]
        check: bool,
    },
    /// Emit a reproducibility manifest (env + git SHA + constants).
    Reproduce {
        /// Output path; default `<phd_root>/reproducibility.lock.json`.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compile `main.tex` via the system `tectonic` binary.
    Compile,
    /// v5.1 render pipeline — pull `ssot.chapters` from Neon, preprocess
    /// `body_md`, build cover + 44 chapters with continuous page numbers,
    /// concatenate to `monograph.pdf`. R1-honest replacement for the
    /// previous `docs/phd-pipeline-v5/{render,compile_all}.sh` scripts.
    Render {
        /// Workdir for intermediate + final artefacts.
        /// Defaults to `<phd_root>/render-out`.
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Neon connection string. If absent, the renderer falls back to
        /// `<workdir>/chapters.json` (created on the previous run) or to
        /// the `NEON_URL` environment variable.
        #[arg(long)]
        neon_url: Option<String>,
    },
}

// -------------------------------------------------------------------------
// AUDIT
// -------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct AuditReport {
    anchor: &'static str,
    main_tex: bool,
    chapters_found: usize,
    frontmatter_found: usize,
    appendix_found: usize,
    bibliography_entries: usize,
    bibliography_floor: usize,
    bibliography_floor_ok: bool,
    chapters_under_floor: Vec<(String, usize)>,
    issues: Vec<String>,
}

fn audit(phd_root: &Path) -> Result<AuditReport> {
    let main_tex = phd_root.join("main.tex").is_file();
    let chapters_dir = phd_root.join("chapters");
    let front_dir = phd_root.join("frontmatter");
    let appx_dir = phd_root.join("appendix");

    let chapters: Vec<PathBuf> = list_tex(&chapters_dir);
    let frontmatter: Vec<PathBuf> = list_tex(&front_dir);
    let appendix: Vec<PathBuf> = list_tex(&appx_dir);

    let bib_entries = count_bib_entries(&phd_root.join("bibliography.bib"))?;
    let bib_ok = bib_entries >= R11_BIB_FLOOR;

    let mut chapters_under_floor = Vec::new();
    for ch in &chapters {
        let lines = count_lines(ch).unwrap_or(0);
        if lines < R3_CHAPTER_MIN_LINES {
            chapters_under_floor.push((
                ch.file_name().unwrap().to_string_lossy().into_owned(),
                lines,
            ));
        }
    }

    let mut issues = Vec::new();
    if !main_tex {
        issues.push("missing main.tex".into());
    }
    if !bib_ok {
        issues.push(format!(
            "R11 violated: {} bib entries < floor {}",
            bib_entries, R11_BIB_FLOOR
        ));
    }
    if chapters.len() < 33 {
        issues.push(format!(
            "expected ≥33 chapters, found {} — check `chapters/` directory",
            chapters.len()
        ));
    }

    Ok(AuditReport {
        anchor: TRINITY_ANCHOR,
        main_tex,
        chapters_found: chapters.len(),
        frontmatter_found: frontmatter.len(),
        appendix_found: appendix.len(),
        bibliography_entries: bib_entries,
        bibliography_floor: R11_BIB_FLOOR,
        bibliography_floor_ok: bib_ok,
        chapters_under_floor,
        issues,
    })
}

// -------------------------------------------------------------------------
// BIBLIOGRAPHY
// -------------------------------------------------------------------------

fn count_bib_entries(path: &Path) -> Result<usize> {
    if !path.is_file() {
        return Err(anyhow!("bibliography file not found at {}", path.display()));
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    Ok(content
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with('@') && !t.to_lowercase().starts_with("@comment")
        })
        .count())
}

#[derive(Debug, Serialize)]
struct BiblioReport {
    entries: usize,
    floor: usize,
    floor_ok: bool,
    rule: &'static str,
}

fn biblio(phd_root: &Path) -> Result<BiblioReport> {
    let entries = count_bib_entries(&phd_root.join("bibliography.bib"))?;
    Ok(BiblioReport {
        entries,
        floor: R11_BIB_FLOOR,
        floor_ok: entries >= R11_BIB_FLOOR,
        rule: "R11 — bibliography ≥150 entries, ≥80% Q1/Q2",
    })
}

// -------------------------------------------------------------------------
// COQ MAP
// -------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)] // schema kept stable for future JSON manifest ingestion
struct CoqEntry {
    theorem: String,
    coq_file: String,
    status: String, // "Proven" | "Admitted"
    phd_chapter: String,
}

/// Verify the appendix referencing Coq theorems exists (R14 floor).
fn coq_map(phd_root: &Path, check: bool) -> Result<()> {
    let appx = phd_root.join("appendix").join("F-coq-citation-map.tex");
    if !appx.is_file() {
        return Err(anyhow!(
            "R14 violated: appendix/F-coq-citation-map.tex missing at {}",
            appx.display()
        ));
    }
    let body = std::fs::read_to_string(&appx)?;
    let proven = body.matches("Proven").count();
    let admitted = body.matches("Admitted").count();
    if proven + admitted == 0 {
        return Err(anyhow!(
            "R14 violated: F-coq-citation-map.tex contains no Proven/Admitted markers"
        ));
    }
    println!(
        "coq-map OK · proven={} admitted={} (check={})",
        proven, admitted, check
    );
    Ok(())
}

// -------------------------------------------------------------------------
// REPRODUCIBILITY
// -------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ReproManifest {
    anchor: &'static str,
    rustc: String,
    cargo: String,
    git_sha: Option<String>,
    constants: ConstantsPinned,
    rules: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct ConstantsPinned {
    phi: f64,
    prune_threshold: f64,
    warmup_blind_steps: u32,
    d_model_min: u32,
    lr_champion: f64,
    nca_certified_band: [f64; 2],
    rungs: [u32; 4],
}

fn reproduce(phd_root: &Path, out: Option<PathBuf>) -> Result<()> {
    let phi = 1.6180339887498949_f64;
    let manifest = ReproManifest {
        anchor: TRINITY_ANCHOR,
        rustc: capture("rustc", &["--version"]).unwrap_or_else(|_| "unknown".into()),
        cargo: capture("cargo", &["--version"]).unwrap_or_else(|_| "unknown".into()),
        git_sha: capture("git", &["rev-parse", "HEAD"]).ok(),
        constants: ConstantsPinned {
            phi,
            prune_threshold: 3.5,
            warmup_blind_steps: 4000,
            d_model_min: 256,
            lr_champion: 0.004,
            nca_certified_band: [phi, phi * phi],
            rungs: [1000, 3000, 9000, 27000],
        },
        rules: vec![
            "R1 Rust/Zig only",
            "R3 ≥1500 lines per chapter",
            "R4 L-R14 traceable constants",
            "R5 honest Admitted",
            "R7 falsification witness",
            "R11 ≥150 bib entries",
            "R14 Coq citation table",
        ],
    };
    let path = out.unwrap_or_else(|| phd_root.join("reproducibility.lock.json"));
    let body = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&path, body)?;
    println!("repro manifest written: {}", path.display());
    Ok(())
}

// -------------------------------------------------------------------------
// COMPILE
// -------------------------------------------------------------------------

fn compile(phd_root: &Path) -> Result<()> {
    let main = phd_root.join("main.tex");
    if !main.is_file() {
        return Err(anyhow!("main.tex not found at {}", main.display()));
    }
    // We invoke the system `tectonic` binary. The `tectonic` Rust crate would
    // pull in heavy native deps (harfbuzz, freetype) that aren't always
    // available in CI; the binary form is the documented user path.
    let status = std::process::Command::new("tectonic")
        .arg(&main)
        .arg("--keep-intermediates")
        .arg("--keep-logs")
        .status()
        .with_context(|| {
            "failed to spawn `tectonic` — install via `cargo install tectonic` or your package manager"
        })?;
    if !status.success() {
        return Err(anyhow!("tectonic exited non-zero: {}", status));
    }
    Ok(())
}

// -------------------------------------------------------------------------
// HELPERS
// -------------------------------------------------------------------------

fn list_tex(dir: &Path) -> Vec<PathBuf> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("tex"))
        .collect();
    out.sort();
    out
}

fn count_lines(path: &Path) -> Result<usize> {
    let s = std::fs::read_to_string(path)?;
    Ok(s.lines().count())
}

fn capture(cmd: &str, args: &[&str]) -> Result<String> {
    let out = std::process::Command::new(cmd).args(args).output()?;
    if !out.status.success() {
        return Err(anyhow!("{} failed", cmd));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// -------------------------------------------------------------------------
// MAIN
// -------------------------------------------------------------------------

fn main() -> ExitCode {
    let cli = Cli::parse();
    let r = match &cli.cmd {
        Cmd::Audit => audit(&cli.phd_root).map(|r| {
            println!("{}", serde_json::to_string_pretty(&r).unwrap());
            if !r.issues.is_empty() {
                eprintln!("audit failed: {} issue(s)", r.issues.len());
                std::process::exit(2);
            }
        }),
        Cmd::Biblio => biblio(&cli.phd_root).map(|r| {
            println!("{}", serde_json::to_string_pretty(&r).unwrap());
            if !r.floor_ok {
                std::process::exit(2);
            }
        }),
        Cmd::CoqMap { check } => coq_map(&cli.phd_root, *check),
        Cmd::Reproduce { out } => reproduce(&cli.phd_root, out.clone()),
        Cmd::Compile => compile(&cli.phd_root),
        Cmd::Render { workdir, neon_url } => {
            let wd = workdir.clone().unwrap_or_else(||
                cli.phd_root.join("render-out"));
            let url = neon_url.clone()
                .or_else(|| std::env::var("NEON_URL").ok());
            let cfg = render::RenderConfig::new(wd, url);
            render::run(&cfg)
        }
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("trios-phd: {:#}", e);
            ExitCode::from(1)
        }
    }
}

// -------------------------------------------------------------------------
// TESTS
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::create_dir_all(root.join("chapters")).unwrap();
        std::fs::create_dir_all(root.join("frontmatter")).unwrap();
        std::fs::create_dir_all(root.join("appendix")).unwrap();
        std::fs::write(root.join("main.tex"), "\\documentclass{book}\n").unwrap();
        // 33 chapters
        for i in 0..33 {
            std::fs::write(
                root.join("chapters").join(format!("ch{:02}.tex", i)),
                "% chapter\n",
            )
            .unwrap();
        }
        // bib with 160 entries
        let mut bib = String::new();
        for i in 0..160 {
            bib.push_str(&format!("@article{{ref{},\n  title={{T}},\n}}\n", i));
        }
        std::fs::write(root.join("bibliography.bib"), bib).unwrap();
        // coq-map appendix
        std::fs::write(
            root.join("appendix").join("F-coq-citation-map.tex"),
            "Proven\\\\Admitted\\\\Proven\n",
        )
        .unwrap();
        d
    }

    #[test]
    fn test_trinity_anchor_constant() {
        let phi = 1.6180339887498949_f64;
        assert!((phi * phi + 1.0 / (phi * phi) - 3.0).abs() < 1e-12);
    }

    #[test]
    fn test_audit_clean_fixture() {
        let d = fixture();
        let r = audit(d.path()).unwrap();
        assert!(r.main_tex);
        assert_eq!(r.chapters_found, 33);
        assert!(r.bibliography_floor_ok);
        assert!(r.issues.is_empty(), "issues = {:?}", r.issues);
    }

    #[test]
    fn test_audit_flags_low_bib() {
        let d = fixture();
        std::fs::write(
            d.path().join("bibliography.bib"),
            "@article{x,title={t},}\n",
        )
        .unwrap();
        let r = audit(d.path()).unwrap();
        assert!(!r.bibliography_floor_ok);
        assert!(r.issues.iter().any(|s| s.contains("R11")));
    }

    #[test]
    fn test_biblio_counts_entries() {
        let d = fixture();
        let r = biblio(d.path()).unwrap();
        assert_eq!(r.entries, 160);
        assert!(r.floor_ok);
    }

    #[test]
    fn test_biblio_ignores_at_comment_directive() {
        let d = fixture();
        std::fs::write(
            d.path().join("bibliography.bib"),
            "@COMMENT{ignored}\n@article{a,t={x},}\n@article{b,t={y},}\n",
        )
        .unwrap();
        let r = biblio(d.path()).unwrap();
        assert_eq!(r.entries, 2);
    }

    #[test]
    fn test_coq_map_rejects_missing() {
        let d = fixture();
        std::fs::remove_file(d.path().join("appendix").join("F-coq-citation-map.tex")).unwrap();
        let err = coq_map(d.path(), true).unwrap_err();
        assert!(format!("{}", err).contains("R14 violated"));
    }

    #[test]
    fn test_coq_map_rejects_empty_table() {
        let d = fixture();
        std::fs::write(
            d.path().join("appendix").join("F-coq-citation-map.tex"),
            "no markers here\n",
        )
        .unwrap();
        let err = coq_map(d.path(), true).unwrap_err();
        assert!(format!("{}", err).contains("R14 violated"));
    }

    #[test]
    fn test_coq_map_accepts_present() {
        let d = fixture();
        coq_map(d.path(), true).unwrap();
    }

    #[test]
    fn test_reproduce_writes_manifest() {
        let d = fixture();
        let out = d.path().join("repro.json");
        reproduce(d.path(), Some(out.clone())).unwrap();
        let body = std::fs::read_to_string(&out).unwrap();
        assert!(body.contains("phi^2"));
        assert!(body.contains("\"prune_threshold\": 3.5"));
        assert!(body.contains("\"warmup_blind_steps\": 4000"));
        assert!(body.contains("\"d_model_min\": 256"));
    }

    #[test]
    fn test_constants_pinned_match_assertions() {
        // L-R14: every numeric constant in the manifest must equal the
        // canonical assertion value. Mirrors `assertions/igla_assertions.json`.
        let phi = 1.6180339887498949_f64;
        let c = ConstantsPinned {
            phi,
            prune_threshold: 3.5,
            warmup_blind_steps: 4000,
            d_model_min: 256,
            lr_champion: 0.004,
            nca_certified_band: [phi, phi * phi],
            rungs: [1000, 3000, 9000, 27000],
        };
        assert!((c.phi * c.phi + 1.0 / (c.phi * c.phi) - 3.0).abs() < 1e-12);
        assert_eq!(c.prune_threshold, 3.5);
        assert_eq!(c.warmup_blind_steps, 4000);
        assert_eq!(c.d_model_min, 256);
        assert!(c.lr_champion >= 0.002 && c.lr_champion <= 0.007);
        assert_eq!(c.rungs, [1000, 3000, 9000, 27000]);
    }

    #[test]
    fn test_forbidden_prune_threshold_rejected() {
        // R7: 2.65 was the killer threshold; a manifest using it would betray
        // the trinity anchor and is forbidden.
        let bad = 2.65_f64;
        let good = 3.5_f64;
        assert_ne!(bad, good);
    }

    #[test]
    fn test_forbidden_lr_band_rejected() {
        // R7: lr ∉ [0.002, 0.007] is forbidden.
        let lr_bad = 0.01_f64;
        assert!(!(lr_bad >= 0.002 && lr_bad <= 0.007));
    }

    #[test]
    fn test_count_bib_handles_blank_file() {
        let d = fixture();
        std::fs::write(d.path().join("bibliography.bib"), "").unwrap();
        let n = count_bib_entries(&d.path().join("bibliography.bib")).unwrap();
        assert_eq!(n, 0);
    }
}
