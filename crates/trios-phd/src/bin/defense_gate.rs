//! Defense gate witness binary (LD lane, ONE SHOT v2.0 §5).
//!
//! R1-compliant Rust replacement for the `assertions/witness/defense_gate.sh`
//! that ONE SHOT v2.0 §5 names. The gate exits non-zero when any of the
//! Phase-C deliverables is missing or below threshold:
//!
//! - `docs/phd/defense/examiner-pack.tex` body fill (≥ 200 non-blank lines, ≥10 sections).
//! - `docs/phd/defense/qa.tex` carries ≥ 30 `\section*{Q…}` blocks.
//! - `docs/phd/defense/slides.tex` carries ≥ 30 frames (counting `\maketitle` as one).
//! - `docs/phd/appendix/F-coq-citation-map.tex` carries ≥ 260 lines (post-#288).
//! - `assertions/seed_results.jsonl` carries ≥ 3 distinct seeds with `bpb < 1.50` (Gate-final).
//!
//! R5 honest: every check uses a finite, content-anchored threshold. No silent
//! flips; failures print a structured diagnosis on stderr.
//!
//! Usage (from repo root):
//! ```sh
//! cargo run -p trios-phd --bin defense_gate
//! ```
//! Exit 0 = pass; exit 1 = at least one gate failed.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Clone, Copy)]
struct GateResult {
    name: &'static str,
    pass: bool,
    detail: &'static str,
}

fn count_lines(path: &Path) -> std::io::Result<usize> {
    let s = fs::read_to_string(path)?;
    Ok(s.lines().count())
}

fn count_pattern(path: &Path, needle: &str) -> std::io::Result<usize> {
    let s = fs::read_to_string(path)?;
    Ok(s.matches(needle).count())
}

fn distinct_seeds_under_bpb(path: &Path, threshold: f64) -> Option<usize> {
    let s = fs::read_to_string(path).ok()?;
    let mut seeds: BTreeSet<i64> = BTreeSet::new();
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let bpb = v.get("bpb").and_then(|x| x.as_f64()).unwrap_or(f64::INFINITY);
        let seed = v.get("seed").and_then(|x| x.as_i64());
        if let Some(s) = seed {
            if bpb < threshold {
                seeds.insert(s);
            }
        }
    }
    Some(seeds.len())
}

fn gate_examiner_pack(repo_root: &Path) -> GateResult {
    let p = repo_root.join("docs/phd/defense/examiner-pack.tex");
    let lines = count_lines(&p).unwrap_or(0);
    let sections = count_pattern(&p, "\\section{").unwrap_or(0);
    let pass = lines >= 200 && sections >= 10;
    GateResult {
        name: "examiner-pack",
        pass,
        detail: if pass { "≥200 lines, ≥10 sections" } else { "BODY MISSING (LD lane open)" },
    }
}

fn gate_qa(repo_root: &Path) -> GateResult {
    let p = repo_root.join("docs/phd/defense/qa.tex");
    let q_count = count_pattern(&p, "\\section*{Q").unwrap_or(0);
    let pass = q_count >= 30;
    GateResult {
        name: "qa",
        pass,
        detail: if pass { "≥30 Q&A pairs" } else { "fewer than 30 Q&A pairs" },
    }
}

fn gate_slides(repo_root: &Path) -> GateResult {
    let p = repo_root.join("docs/phd/defense/slides.tex");
    let frames = count_pattern(&p, "\\begin{frame}").unwrap_or(0);
    let titles = count_pattern(&p, "\\maketitle").unwrap_or(0);
    let total = frames + titles;
    let pass = total >= 30;
    GateResult {
        name: "slides",
        pass,
        detail: if pass { "≥30 frames" } else { "fewer than 30 frames" },
    }
}

fn gate_appendix_f(repo_root: &Path) -> GateResult {
    let p = repo_root.join("docs/phd/appendix/F-coq-citation-map.tex");
    let lines = count_lines(&p).unwrap_or(0);
    let pass = lines >= 260;
    GateResult {
        name: "appendix-F",
        pass,
        detail: if pass {
            "≥260 lines (post-#288)"
        } else {
            "62-byte stub on main; PR #288 restoration pending"
        },
    }
}

fn gate_seed_ledger(repo_root: &Path) -> GateResult {
    let p = repo_root.join("assertions/seed_results.jsonl");
    let count = distinct_seeds_under_bpb(&p, 1.50).unwrap_or(0);
    let pass = count >= 3;
    GateResult {
        name: "seed-ledger",
        pass,
        detail: if pass {
            "≥3 distinct seeds with bpb<1.50 (Gate-final)"
        } else {
            "fewer than 3 distinct seeds with bpb<1.50 (L-h3 lane open)"
        },
    }
}

fn locate_repo_root() -> PathBuf {
    // Walk up from the binary's CWD until we find docs/phd/ or fall back to ".".
    let mut cur = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if cur.join("docs/phd").is_dir() && cur.join("assertions").is_dir() {
            return cur;
        }
        if !cur.pop() {
            return PathBuf::from(".");
        }
    }
}

fn main() -> ExitCode {
    let repo_root = locate_repo_root();
    let gates = [
        gate_examiner_pack(&repo_root),
        gate_qa(&repo_root),
        gate_slides(&repo_root),
        gate_appendix_f(&repo_root),
        gate_seed_ledger(&repo_root),
    ];
    println!("Defense gate witness — repo {}", repo_root.display());
    let mut all_pass = true;
    for g in &gates {
        let mark = if g.pass { "✅" } else { "❌" };
        println!("  {mark} {:<14}  {}", g.name, g.detail);
        if !g.pass {
            all_pass = false;
        }
    }
    if all_pass {
        println!("\nGate-state: PASS — defense package + appendix F + seed ledger ready.");
        ExitCode::SUCCESS
    } else {
        eprintln!("\nGate-state: FAIL — at least one Phase-C/Phase-A/Phase-B item below threshold.");
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_seeds_under_bpb_handles_empty() {
        let t = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(t.path(), "").unwrap();
        assert_eq!(distinct_seeds_under_bpb(t.path(), 1.50), Some(0));
    }

    #[test]
    fn distinct_seeds_under_bpb_counts_unique() {
        let t = tempfile::NamedTempFile::new().unwrap();
        let body = r#"{"seed":42,"bpb":1.49,"sha":"abc"}
{"seed":42,"bpb":1.20,"sha":"def"}
{"seed":43,"bpb":1.40,"sha":"ghi"}
{"seed":44,"bpb":1.49,"sha":"jkl"}
"#;
        std::fs::write(t.path(), body).unwrap();
        assert_eq!(distinct_seeds_under_bpb(t.path(), 1.50), Some(3));
    }

    #[test]
    fn distinct_seeds_under_bpb_rejects_above_threshold() {
        let t = tempfile::NamedTempFile::new().unwrap();
        let body = r#"{"seed":43,"bpb":2.24,"sha":"abc"}
"#;
        std::fs::write(t.path(), body).unwrap();
        assert_eq!(distinct_seeds_under_bpb(t.path(), 1.50), Some(0));
    }

    #[test]
    fn distinct_seeds_under_bpb_skips_blank_and_comments() {
        let t = tempfile::NamedTempFile::new().unwrap();
        let body = "# header\n\n{\"seed\":43,\"bpb\":1.49,\"sha\":\"abc\"}\n";
        std::fs::write(t.path(), body).unwrap();
        assert_eq!(distinct_seeds_under_bpb(t.path(), 1.50), Some(1));
    }
}
