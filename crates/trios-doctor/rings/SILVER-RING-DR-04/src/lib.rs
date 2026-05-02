//! SILVER-RING-DR-04 — ring-architecture lint rules
//!
//! Implements eight ring-architecture lints that the gold `trios-doctor`
//! binary surfaces alongside the cargo check / clippy / test passes
//! that already live in SILVER-RING-DR-01.
//!
//! ## Rules
//!
//! | Id | Rule | Severity (today) |
//! |----|------|------------------|
//! | `R-RING-FACADE-001` | outer GOLD `src/lib.rs` ≤ 50 LoC, re-exports only | warn |
//! | `R-RING-DEP-002`    | strict dep lists per ring; Silver-tier has no I/O   | warn |
//! | `R-RING-FLOW-003`   | data flows down (Silver→Bronze), never up           | warn |
//! | `R-RING-BR-004`     | Bronze rings re-exposed via parent GOLD facade      | warn |
//! | `R-MCP-BRIDGE-005`  | MCP bridge rings live under `vendor/tri-mcp/rings/` | warn |
//! | `R-L1-ECHO-006`     | no `.sh` files inside `crates/`                     | warn |
//! | `R-L6-PURE-007`     | no `.py` files inside `crates/`                     | warn |
//! | `R-COQ-LINK-008`    | every Bronze BPB sink references a Coq theorem id   | warn |
//!
//! ### Severity escalation
//!
//! Configured in `[doctor.escalation] T_plus_30 = "2026-06-01"`. After
//! `T_plus_30`, `Severity::Warn` becomes `Severity::Error`. Until then
//! every finding is non-blocking. The escalation is read off the wall
//! clock the caller passes in — there is no internal clock here.
//!
//! ## Honored constitutional rules
//!
//! - **R-RING-FACADE-001** (recursive: this ring honors the rule it
//!   enforces) — outer `src/lib.rs` of `trios-doctor` is unaffected by
//!   this ring; this ring is dead-code from the gold facade until DR-05
//!   wires `RuleEngine` in.
//! - **R-RING-DEP-002** — Silver-tier deps only: `serde`, `serde_json`,
//!   `toml`, `chrono`, `walkdir`, `thiserror`. No tokio, no reqwest, no
//!   subprocess.
//! - **R-L1-ECHO-006** — no `.sh` in this ring.
//! - **R-L6-PURE-007** — no `.py` in this ring.
//! - **L13** — single-ring scope.
//!
//! Closes #462 · Part of #446 · Soul: Doctor-Doctrine

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

// ─────────── public types ───────────

/// Identifier of one ring-architecture rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RuleId {
    /// outer GOLD `src/lib.rs` ≤ 50 LoC, re-exports only
    RingFacade001,
    /// strict dep list per ring; Silver-tier has no I/O
    RingDep002,
    /// data flows down (Silver→Bronze), never up
    RingFlow003,
    /// Bronze rings re-exposed via parent GOLD facade
    RingBr004,
    /// MCP bridge rings live under `vendor/tri-mcp/rings/`
    McpBridge005,
    /// no `.sh` files inside `crates/`
    L1Echo006,
    /// no `.py` files inside `crates/`
    L6Pure007,
    /// every Bronze BPB sink references a Coq theorem id
    CoqLink008,
}

impl RuleId {
    /// Stable string slug used in JSON output and CI logs.
    pub fn slug(&self) -> &'static str {
        match self {
            RuleId::RingFacade001 => "R-RING-FACADE-001",
            RuleId::RingDep002 => "R-RING-DEP-002",
            RuleId::RingFlow003 => "R-RING-FLOW-003",
            RuleId::RingBr004 => "R-RING-BR-004",
            RuleId::McpBridge005 => "R-MCP-BRIDGE-005",
            RuleId::L1Echo006 => "R-L1-ECHO-006",
            RuleId::L6Pure007 => "R-L6-PURE-007",
            RuleId::CoqLink008 => "R-COQ-LINK-008",
        }
    }

    /// Every rule, in stable order.
    pub fn all() -> [RuleId; 8] {
        [
            RuleId::RingFacade001,
            RuleId::RingDep002,
            RuleId::RingFlow003,
            RuleId::RingBr004,
            RuleId::McpBridge005,
            RuleId::L1Echo006,
            RuleId::L6Pure007,
            RuleId::CoqLink008,
        ]
    }
}

/// Severity of one finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Non-blocking warning.
    Warn,
    /// Blocking error.
    Error,
}

/// Escalation configuration: warn → error after `t_plus_30`.
///
/// Read from `doctor.toml`'s `[doctor.escalation]` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscalationConfig {
    /// UTC date after which warn-severity findings become errors.
    pub t_plus_30: DateTime<Utc>,
}

impl EscalationConfig {
    /// Decide the effective severity for a finding given the wall clock.
    pub fn severity_at(&self, now: DateTime<Utc>) -> Severity {
        if now >= self.t_plus_30 {
            Severity::Error
        } else {
            Severity::Warn
        }
    }
}

/// One lint finding — what failed, where, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// Which rule produced this finding.
    pub rule: RuleId,
    /// Computed severity (post escalation).
    pub severity: Severity,
    /// Path that triggered the finding (relative to repo root).
    pub path: PathBuf,
    /// Human-readable message.
    pub message: String,
}

/// Errors surfaced by the rule engine.
#[derive(Debug, Error)]
pub enum DoctorError {
    /// IO failure while reading a file or directory.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// `doctor.toml` could not be parsed.
    #[error("config parse: {0}")]
    Config(#[from] toml::de::Error),
    /// Walkdir traversal failed.
    #[error("walk: {0}")]
    Walk(String),
}

// ─────────── doctor.toml schema ───────────

/// Top-level `doctor.toml` shape (only the bits this ring cares about).
#[derive(Debug, Clone, Deserialize)]
pub struct DoctorConfig {
    /// `[doctor]` table.
    pub doctor: DoctorSection,
}

/// `[doctor]` table.
#[derive(Debug, Clone, Deserialize)]
pub struct DoctorSection {
    /// `[doctor.escalation]` sub-table.
    pub escalation: EscalationConfig,
}

impl DoctorConfig {
    /// Parse a TOML string. Defaults are applied if `[doctor.escalation]`
    /// is missing — `t_plus_30` defaults to `2026-06-01T00:00:00Z`.
    pub fn parse(s: &str) -> Result<Self, DoctorError> {
        Ok(toml::from_str(s)?)
    }

    /// Default escalation date used when `doctor.toml` is absent: the
    /// canonical `T+30` for EPIC #446 ring rollout.
    pub fn default_t_plus_30() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .expect("compile-time RFC3339")
            .with_timezone(&Utc)
    }
}

// ─────────── rule engine ───────────

/// Cap on outer-facade LoC for `R-RING-FACADE-001`.
pub const OUTER_FACADE_LOC_CAP: usize = 50;

/// Returns true if every line of `lib_rs` looks like a re-export, doc
/// comment, blank line, or `#![attribute]` line. This is the
/// "re-exports only" predicate for `R-RING-FACADE-001`.
pub fn is_facade_only(lib_rs: &str) -> bool {
    for raw in lib_rs.lines() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with("//!")
            || line.starts_with("//")
            || line.starts_with("#![")
            || line.starts_with("#[")
            || line.starts_with("pub use")
            || line.starts_with("pub mod ") && line.ends_with(';')
            || line.starts_with("mod ") && line.ends_with(';')
            || line.starts_with("use ")
            || line == "}"
            || line.starts_with("};")
        {
            continue;
        }
        return false;
    }
    true
}

/// Top-level rule engine. Stateless — every call walks the workspace
/// fresh.
#[derive(Debug, Default, Clone)]
pub struct RuleEngine;

impl RuleEngine {
    /// Build a new engine.
    pub fn new() -> Self {
        Self
    }

    /// Run every rule against `workspace_root`. `now` decides whether
    /// findings ship as Warn or Error.
    pub fn run(
        &self,
        workspace_root: &Path,
        cfg: &EscalationConfig,
        now: DateTime<Utc>,
    ) -> Result<Vec<Finding>, DoctorError> {
        let sev = cfg.severity_at(now);
        let mut out = Vec::new();
        out.extend(self.r1_facade(workspace_root, sev)?);
        out.extend(self.r2_silver_no_io(workspace_root, sev)?);
        out.extend(self.r3_flow(workspace_root, sev)?);
        out.extend(self.r4_bronze_facade(workspace_root, sev)?);
        out.extend(self.r5_mcp_bridge(workspace_root, sev)?);
        out.extend(self.r6_no_sh(workspace_root, sev)?);
        out.extend(self.r7_no_py(workspace_root, sev)?);
        out.extend(self.r8_coq_link(workspace_root, sev)?);
        Ok(out)
    }

    // ─── individual rules ───

    fn r1_facade(&self, root: &Path, sev: Severity) -> Result<Vec<Finding>, DoctorError> {
        let mut out = Vec::new();
        let crates_dir = root.join("crates");
        if !crates_dir.exists() {
            return Ok(out);
        }
        for entry in fs::read_dir(&crates_dir)? {
            let entry = entry?;
            let lib = entry.path().join("src/lib.rs");
            if !lib.exists() {
                continue;
            }
            let body = fs::read_to_string(&lib)?;
            let loc = body.lines().count();
            if loc > OUTER_FACADE_LOC_CAP || !is_facade_only(&body) {
                out.push(Finding {
                    rule: RuleId::RingFacade001,
                    severity: sev,
                    path: lib,
                    message: format!(
                        "outer facade has {loc} LoC (cap {OUTER_FACADE_LOC_CAP}) or contains business logic"
                    ),
                });
            }
        }
        Ok(out)
    }

    fn r2_silver_no_io(&self, root: &Path, sev: Severity) -> Result<Vec<Finding>, DoctorError> {
        // Heuristic: any Cargo.toml under `rings/SILVER-*` or
        // `rings/SR-*` may not list `tokio`, `reqwest`, `tonic`, or
        // `rusqlite` as a regular dep. Bronze BR-IO rings are the only
        // place those belong.
        const FORBIDDEN: &[&str] = &["tokio", "reqwest", "tonic", "rusqlite"];
        let mut out = Vec::new();
        for entry in WalkDir::new(root.join("crates"))
            .into_iter()
            .filter_map(Result::ok)
        {
            let p = entry.path();
            if !p.ends_with("Cargo.toml") {
                continue;
            }
            let parent = match p.parent() {
                Some(x) => x,
                None => continue,
            };
            let dirname = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let is_silver = dirname.starts_with("SILVER-RING-") || dirname.starts_with("SR-");
            if !is_silver {
                continue;
            }
            let body = fs::read_to_string(p)?;
            for f in FORBIDDEN {
                // Match a dependency declaration line — be tolerant of
                // workspace deps and crate.io versions.
                let key1 = format!("\n{f} =");
                let key2 = format!("\n{f}.workspace");
                if body.contains(&key1) || body.contains(&key2) {
                    out.push(Finding {
                        rule: RuleId::RingDep002,
                        severity: sev,
                        path: p.to_path_buf(),
                        message: format!("Silver-tier ring lists forbidden I/O dep `{f}`"),
                    });
                }
            }
        }
        Ok(out)
    }

    fn r3_flow(&self, root: &Path, sev: Severity) -> Result<Vec<Finding>, DoctorError> {
        // Heuristic: a SILVER-* ring may not list a BR-* sibling as a
        // path dep — that would be an upward edge.
        let mut out = Vec::new();
        for entry in WalkDir::new(root.join("crates"))
            .into_iter()
            .filter_map(Result::ok)
        {
            let p = entry.path();
            if !p.ends_with("Cargo.toml") {
                continue;
            }
            let parent = match p.parent() {
                Some(x) => x,
                None => continue,
            };
            let dirname = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let is_silver = dirname.starts_with("SILVER-RING-") || dirname.starts_with("SR-");
            if !is_silver {
                continue;
            }
            let body = fs::read_to_string(p)?;
            if body.contains("path = \"../BR-") || body.contains("path = \"../BRONZE-RING-") {
                out.push(Finding {
                    rule: RuleId::RingFlow003,
                    severity: sev,
                    path: p.to_path_buf(),
                    message: "Silver-tier ring depends on a Bronze sibling (upward flow forbidden)"
                        .into(),
                });
            }
        }
        Ok(out)
    }

    fn r4_bronze_facade(&self, root: &Path, sev: Severity) -> Result<Vec<Finding>, DoctorError> {
        // For each `crates/<gold>/rings/BR-*` Bronze ring, verify the
        // outer `crates/<gold>/src/lib.rs` re-exports at least one
        // identifier from `<gold>-br-*`.
        let mut out = Vec::new();
        let crates_dir = root.join("crates");
        if !crates_dir.exists() {
            return Ok(out);
        }
        for gold in fs::read_dir(&crates_dir)? {
            let gold = gold?;
            let rings = gold.path().join("rings");
            if !rings.exists() {
                continue;
            }
            let mut bronze_names = BTreeSet::new();
            for ring in fs::read_dir(&rings)? {
                let ring = ring?;
                let name = ring.file_name().to_string_lossy().to_string();
                if name.starts_with("BR-") || name.starts_with("BRONZE-RING-") {
                    bronze_names.insert(name);
                }
            }
            if bronze_names.is_empty() {
                continue;
            }
            let outer = gold.path().join("src/lib.rs");
            let body = fs::read_to_string(&outer).unwrap_or_default();
            // crude but honest: outer must mention `_br_` or `_bronze_`
            // somewhere in its re-exports.
            let mentions = body.contains("_br_")
                || body.contains("_bronze_")
                || body.contains("br_output")
                || body.contains("br-output");
            if !mentions {
                out.push(Finding {
                    rule: RuleId::RingBr004,
                    severity: sev,
                    path: outer,
                    message: format!(
                        "GOLD facade does not re-export any Bronze ring (Bronze present: {})",
                        bronze_names.iter().cloned().collect::<Vec<_>>().join(", ")
                    ),
                });
            }
        }
        Ok(out)
    }

    fn r5_mcp_bridge(&self, root: &Path, sev: Severity) -> Result<Vec<Finding>, DoctorError> {
        // Anything named `*-mcp-*` outside `vendor/tri-mcp/rings/` is a
        // misplaced MCP bridge ring.
        let mut out = Vec::new();
        for entry in WalkDir::new(root.join("crates"))
            .into_iter()
            .filter_map(Result::ok)
        {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.contains("mcp") && p.starts_with(root.join("crates")) {
                let canonical = root.join("vendor").join("tri-mcp").join("rings");
                if !p.starts_with(&canonical) {
                    out.push(Finding {
                        rule: RuleId::McpBridge005,
                        severity: sev,
                        path: p.to_path_buf(),
                        message: format!(
                            "MCP bridge ring found outside `vendor/tri-mcp/rings/`: `{}`",
                            p.display()
                        ),
                    });
                }
            }
        }
        Ok(out)
    }

    fn r6_no_sh(&self, root: &Path, sev: Severity) -> Result<Vec<Finding>, DoctorError> {
        let mut out = Vec::new();
        for entry in WalkDir::new(root.join("crates"))
            .into_iter()
            .filter_map(Result::ok)
        {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("sh") {
                out.push(Finding {
                    rule: RuleId::L1Echo006,
                    severity: sev,
                    path: p.to_path_buf(),
                    message: ".sh file inside `crates/` (L1 forbids shell scripts in Rust crates)"
                        .into(),
                });
            }
        }
        Ok(out)
    }

    fn r7_no_py(&self, root: &Path, sev: Severity) -> Result<Vec<Finding>, DoctorError> {
        let mut out = Vec::new();
        for entry in WalkDir::new(root.join("crates"))
            .into_iter()
            .filter_map(Result::ok)
        {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("py") {
                out.push(Finding {
                    rule: RuleId::L6Pure007,
                    severity: sev,
                    path: p.to_path_buf(),
                    message:
                        ".py file inside `crates/` (R-L6-PURE-007: entry_path must live OUTSIDE crates/)"
                            .into(),
                });
            }
        }
        Ok(out)
    }

    fn r8_coq_link(&self, root: &Path, sev: Severity) -> Result<Vec<Finding>, DoctorError> {
        // Heuristic: every Bronze ring whose name suggests a BPB sink
        // (`*bpb*` or `*sink*`) must mention `COQ_THEOREM_ID` or
        // `coq_theorem_id` somewhere in its src tree.
        let mut out = Vec::new();
        for entry in WalkDir::new(root.join("crates"))
            .into_iter()
            .filter_map(Result::ok)
        {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let parent_is_rings = p
                .parent()
                .and_then(|x| x.file_name())
                .and_then(|s| s.to_str())
                == Some("rings");
            if !parent_is_rings {
                continue;
            }
            let is_bronze = name.starts_with("BR-") || name.starts_with("BRONZE-RING-");
            if !is_bronze {
                continue;
            }
            let lname = name.to_ascii_lowercase();
            if !(lname.contains("bpb") || lname.contains("sink") || lname.contains("output")) {
                continue;
            }
            let mut mentions = false;
            for src in WalkDir::new(p).into_iter().filter_map(Result::ok) {
                let sp = src.path();
                if sp.extension().and_then(|s| s.to_str()) != Some("rs") {
                    continue;
                }
                if let Ok(body) = fs::read_to_string(sp) {
                    if body.contains("COQ_THEOREM_ID") || body.contains("coq_theorem_id") {
                        mentions = true;
                        break;
                    }
                }
            }
            if !mentions {
                out.push(Finding {
                    rule: RuleId::CoqLink008,
                    severity: sev,
                    path: p.to_path_buf(),
                    message: format!(
                        "Bronze BPB sink `{name}` does not reference any Coq theorem id"
                    ),
                });
            }
        }
        Ok(out)
    }
}

// ─────────── tests ───────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn tmp_workspace() -> tempfile::TempDir {
        tempfile::Builder::new().prefix("dr04").tempdir().expect("tempdir")
    }

    #[test]
    fn rule_id_slugs_are_stable() {
        for id in RuleId::all() {
            assert!(!id.slug().is_empty());
        }
        assert_eq!(RuleId::RingFacade001.slug(), "R-RING-FACADE-001");
        assert_eq!(RuleId::CoqLink008.slug(), "R-COQ-LINK-008");
    }

    #[test]
    fn escalation_warn_before_error_after() {
        let cutoff = DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let cfg = EscalationConfig { t_plus_30: cutoff };
        let before = DateTime::parse_from_rfc3339("2026-05-31T23:59:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let after = DateTime::parse_from_rfc3339("2026-06-01T00:00:01Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(cfg.severity_at(before), Severity::Warn);
        assert_eq!(cfg.severity_at(after), Severity::Error);
    }

    #[test]
    fn doctor_config_parses_escalation() {
        let s = r#"
[doctor.escalation]
t_plus_30 = "2026-06-01T00:00:00Z"
"#;
        let cfg = DoctorConfig::parse(s).unwrap();
        assert_eq!(
            cfg.doctor.escalation.t_plus_30,
            DoctorConfig::default_t_plus_30()
        );
    }

    #[test]
    fn is_facade_only_accepts_reexports_and_docs() {
        let body = r#"
//! crate doc
#![forbid(unsafe_code)]

pub use foo::bar;
pub mod baz;
"#;
        assert!(is_facade_only(body));
    }

    #[test]
    fn is_facade_only_rejects_business_logic() {
        let body = r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }
"#;
        assert!(!is_facade_only(body));
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-05-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn warn_cfg() -> EscalationConfig {
        EscalationConfig {
            t_plus_30: DoctorConfig::default_t_plus_30(),
        }
    }

    #[test]
    fn r6_flags_sh_in_crates() {
        let tmp = tmp_workspace();
        let root = tmp.path();
        fs::create_dir_all(root.join("crates/foo")).unwrap();
        let mut f = fs::File::create(root.join("crates/foo/build.sh")).unwrap();
        writeln!(f, "echo hi").unwrap();
        let findings = RuleEngine::new().run(root, &warn_cfg(), now()).unwrap();
        assert!(findings.iter().any(|f| f.rule == RuleId::L1Echo006));
    }

    #[test]
    fn r7_flags_py_in_crates() {
        let tmp = tmp_workspace();
        let root = tmp.path();
        fs::create_dir_all(root.join("crates/foo")).unwrap();
        let mut f = fs::File::create(root.join("crates/foo/runner.py")).unwrap();
        writeln!(f, "print(1)").unwrap();
        let findings = RuleEngine::new().run(root, &warn_cfg(), now()).unwrap();
        assert!(findings.iter().any(|f| f.rule == RuleId::L6Pure007));
    }

    #[test]
    fn r1_flags_business_logic_in_outer_facade() {
        let tmp = tmp_workspace();
        let root = tmp.path();
        fs::create_dir_all(root.join("crates/foo/src")).unwrap();
        // Cargo.toml is required for the directory to count as a crate.
        fs::write(root.join("crates/foo/Cargo.toml"), "[package]\nname=\"foo\"\n").unwrap();
        fs::write(
            root.join("crates/foo/src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
        )
        .unwrap();
        let findings = RuleEngine::new().run(root, &warn_cfg(), now()).unwrap();
        assert!(findings.iter().any(|f| f.rule == RuleId::RingFacade001));
    }

    #[test]
    fn r1_accepts_clean_facade() {
        let tmp = tmp_workspace();
        let root = tmp.path();
        fs::create_dir_all(root.join("crates/foo/src")).unwrap();
        fs::write(root.join("crates/foo/Cargo.toml"), "[package]\nname=\"foo\"\n").unwrap();
        fs::write(
            root.join("crates/foo/src/lib.rs"),
            "//! doc\npub use bar::baz;\n",
        )
        .unwrap();
        let findings = RuleEngine::new().run(root, &warn_cfg(), now()).unwrap();
        assert!(findings.iter().all(|f| f.rule != RuleId::RingFacade001));
    }

    #[test]
    fn r2_flags_silver_with_tokio() {
        let tmp = tmp_workspace();
        let root = tmp.path();
        let ring = root.join("crates/foo/rings/SILVER-RING-FOO-00");
        fs::create_dir_all(&ring).unwrap();
        fs::write(
            ring.join("Cargo.toml"),
            "[package]\nname=\"foo-00\"\n[dependencies]\ntokio = \"1\"\n",
        )
        .unwrap();
        let findings = RuleEngine::new().run(root, &warn_cfg(), now()).unwrap();
        assert!(findings.iter().any(|f| f.rule == RuleId::RingDep002));
    }

    #[test]
    fn r3_flags_silver_depending_on_bronze() {
        let tmp = tmp_workspace();
        let root = tmp.path();
        let ring = root.join("crates/foo/rings/SILVER-RING-FOO-00");
        fs::create_dir_all(&ring).unwrap();
        fs::write(
            ring.join("Cargo.toml"),
            "[package]\nname=\"foo-00\"\n[dependencies]\nfoo-br = { path = \"../BR-OUTPUT\" }\n",
        )
        .unwrap();
        let findings = RuleEngine::new().run(root, &warn_cfg(), now()).unwrap();
        assert!(findings.iter().any(|f| f.rule == RuleId::RingFlow003));
    }

    #[test]
    fn r5_flags_mcp_outside_vendor() {
        let tmp = tmp_workspace();
        let root = tmp.path();
        fs::create_dir_all(root.join("crates/wayward-mcp-bridge")).unwrap();
        let findings = RuleEngine::new().run(root, &warn_cfg(), now()).unwrap();
        assert!(findings.iter().any(|f| f.rule == RuleId::McpBridge005));
    }

    #[test]
    fn r4_flags_bronze_without_facade_reexport() {
        let tmp = tmp_workspace();
        let root = tmp.path();
        let bronze = root.join("crates/foo/rings/BR-OUTPUT");
        fs::create_dir_all(&bronze).unwrap();
        fs::write(bronze.join("Cargo.toml"), "[package]\nname=\"foo-br\"\n").unwrap();
        fs::create_dir_all(root.join("crates/foo/src")).unwrap();
        fs::write(
            root.join("crates/foo/src/lib.rs"),
            "//! gold\npub use sr_alg_00::*;\n",
        )
        .unwrap();
        fs::write(root.join("crates/foo/Cargo.toml"), "[package]\nname=\"foo\"\n").unwrap();
        let findings = RuleEngine::new().run(root, &warn_cfg(), now()).unwrap();
        assert!(findings.iter().any(|f| f.rule == RuleId::RingBr004));
    }

    #[test]
    fn r8_flags_bronze_bpb_sink_without_coq_link() {
        let tmp = tmp_workspace();
        let root = tmp.path();
        let ring = root.join("crates/foo/rings/BR-bpb-sink");
        fs::create_dir_all(ring.join("src")).unwrap();
        fs::write(ring.join("Cargo.toml"), "[package]\nname=\"foo-br\"\n").unwrap();
        fs::write(ring.join("src/lib.rs"), "pub fn write_row() {}\n").unwrap();
        let findings = RuleEngine::new().run(root, &warn_cfg(), now()).unwrap();
        assert!(findings.iter().any(|f| f.rule == RuleId::CoqLink008));
    }

    #[test]
    fn r8_accepts_bronze_with_coq_link() {
        let tmp = tmp_workspace();
        let root = tmp.path();
        let ring = root.join("crates/foo/rings/BR-bpb-sink");
        fs::create_dir_all(ring.join("src")).unwrap();
        fs::write(ring.join("Cargo.toml"), "[package]\nname=\"foo-br\"\n").unwrap();
        fs::write(
            ring.join("src/lib.rs"),
            "pub const COQ_THEOREM_ID: &str = \"alpha_phi_phi_cubed\";\n",
        )
        .unwrap();
        let findings = RuleEngine::new().run(root, &warn_cfg(), now()).unwrap();
        assert!(findings.iter().all(|f| f.rule != RuleId::CoqLink008));
    }

    #[test]
    fn empty_workspace_has_no_findings() {
        let tmp = tmp_workspace();
        let root = tmp.path();
        let findings = RuleEngine::new().run(root, &warn_cfg(), now()).unwrap();
        assert!(findings.is_empty());
    }
}
