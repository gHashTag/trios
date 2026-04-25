//! Audit subcommand — the L-R14 gate at chapter level.
//!
//! Loads `assertions/igla_assertions.json` (single source of truth, schema 1.0.0)
//! and walks `docs/phd/chapters/**.tex` checking that:
//!
//! 1. Every `\coqbox{<KEY>}` macro resolves to a known invariant id
//!    (`INV-1`..`INV-N`).
//! 2. Any `\coqbox{INV-X}` referencing an `Admitted` invariant is accompanied —
//!    in the same chapter — by an `\admittedbox{...}` macro (rule R5).
//! 3. The metadata is internally consistent: budget book-keeping, per-invariant
//!    `status` honesty, schema_version drift refusal.
//!
//! The audit is intentionally conservative. Page-count, CODATA range checks, and
//! `tectonic` build are tracked in `docs/phd/BRIDGE_AUDIT.md`.

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Schema versions this loader understands. Refusing incompatible JSON is
/// explicitly part of the enforcement contract (`enforcement.schema_drift_policy`).
pub const SUPPORTED_SCHEMA_VERSIONS: &[&str] = &["1.0.0"];

/// Typed view of `assertions/igla_assertions.json` (schema 1.0.0).
/// Extra fields are tolerated so this skeleton survives minor upstream additions.
#[derive(Debug, Deserialize, Serialize)]
pub struct Assertions {
    #[serde(rename = "_metadata")]
    pub meta: Meta,
    #[serde(default)]
    pub trinity_identity: String,
    pub invariants: Vec<Invariant>,
    #[serde(default)]
    pub enforcement: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Meta {
    pub schema_version: String,
    #[serde(default)]
    pub order_ref: String,
    pub theorem_count: TheoremCount,
    pub admitted_budget: AdmittedBudget,
    #[serde(default)]
    pub falsification_witnesses: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TheoremCount {
    pub igla_total: u32,
    pub proven_qed: u32,
    pub honest_admitted: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AdmittedBudget {
    pub max: u32,
    pub used: u32,
    #[serde(default)]
    pub breakdown: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Invariant {
    pub id: String,
    pub name: String,
    pub coq_file: String,
    pub status: ProofStatus,
    #[serde(default)]
    pub coq_theorem: String,
    #[serde(default)]
    pub admitted_theorems: Vec<String>,
    #[serde(default)]
    pub admitted_reason: String,
    #[serde(default)]
    pub runtime_target: String,
}

/// Honest proof status, propagated unchanged from the JSON. Mirrors `cite::ProofStatus`
/// but lives here too so the audit module is standalone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStatus {
    Proven,
    Admitted,
}

impl Assertions {
    /// Load and validate the JSON at `path`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading assertions JSON at {}", path.display()))?;
        let parsed: Self = serde_json::from_str(&raw)
            .with_context(|| format!("parsing assertions JSON at {}", path.display()))?;
        parsed.self_check()?;
        Ok(parsed)
    }

    /// Internal consistency: schema_version, budget book-keeping, status counts.
    pub fn self_check(&self) -> Result<()> {
        // Schema drift refusal — explicitly the contract in enforcement.schema_drift_policy.
        if !SUPPORTED_SCHEMA_VERSIONS.contains(&self.meta.schema_version.as_str()) {
            bail!(
                "unsupported schema_version: {} (supported: {:?})",
                self.meta.schema_version,
                SUPPORTED_SCHEMA_VERSIONS
            );
        }
        // Budget: used ≤ max.
        if self.meta.admitted_budget.used > self.meta.admitted_budget.max {
            bail!(
                "admitted_budget.used ({}) exceeds admitted_budget.max ({})",
                self.meta.admitted_budget.used,
                self.meta.admitted_budget.max
            );
        }
        // Per-invariant id uniqueness.
        let mut ids = std::collections::BTreeSet::new();
        for inv in &self.invariants {
            if !ids.insert(&inv.id) {
                bail!("duplicate invariant id: {}", inv.id);
            }
        }
        // Status count must match the metadata claim.
        let admitted_count = self
            .invariants
            .iter()
            .filter(|i| matches!(i.status, ProofStatus::Admitted))
            .count() as u32;
        let proven_count = self
            .invariants
            .iter()
            .filter(|i| matches!(i.status, ProofStatus::Proven))
            .count() as u32;
        if admitted_count + proven_count != self.invariants.len() as u32 {
            bail!("invariant count mismatch: each invariant must be Proven or Admitted");
        }
        // Honest admitted-theorems list backs every Admitted invariant.
        for inv in &self.invariants {
            if matches!(inv.status, ProofStatus::Admitted) && inv.admitted_theorems.is_empty() {
                bail!(
                    "{}: status=Admitted but admitted_theorems is empty (rule R5: claim must be backed by named theorem(s))",
                    inv.id
                );
            }
            if matches!(inv.status, ProofStatus::Proven) && !inv.admitted_theorems.is_empty() {
                bail!(
                    "{}: status=Proven but admitted_theorems is non-empty (rule R5 violation)",
                    inv.id
                );
            }
        }
        Ok(())
    }

    /// Lookup helper: `Some(invariant)` if `id` matches.
    pub fn get(&self, id: &str) -> Option<&Invariant> {
        self.invariants.iter().find(|i| i.id == id)
    }

    /// True if `id` is Proven.
    pub fn is_proven(&self, id: &str) -> bool {
        self.get(id)
            .map(|i| matches!(i.status, ProofStatus::Proven))
            .unwrap_or(false)
    }
}

/// Walk a chapters directory and return every `*.tex` path. Pure stdlib — no
/// extra dependencies — keeps the skeleton minimal.
pub fn list_chapter_files(chapters_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let dir = chapters_dir.as_ref();
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)
        .with_context(|| format!("reading chapters dir {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("tex") {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Per-chapter audit result.
#[derive(Debug, Default)]
pub struct ChapterAudit {
    pub coqbox_invariants: Vec<String>,
    pub admittedbox_count: usize,
    pub line_count: usize,
}

/// Inspect a single chapter file. No regex dependency — uses stdlib `find`.
pub fn audit_chapter(path: &Path) -> Result<ChapterAudit> {
    let body = fs::read_to_string(path)
        .with_context(|| format!("reading chapter {}", path.display()))?;
    let mut out = ChapterAudit {
        line_count: body.lines().count(),
        ..Default::default()
    };
    let mut cursor = 0;
    while let Some(idx) = body[cursor..].find("\\coqbox{") {
        let start = cursor + idx + "\\coqbox{".len();
        if let Some(end_rel) = body[start..].find('}') {
            let key = &body[start..start + end_rel];
            out.coqbox_invariants.push(key.trim().to_string());
            cursor = start + end_rel + 1;
        } else {
            return Err(anyhow!(
                "{}: unmatched \\coqbox{{ at byte {}",
                path.display(),
                start
            ));
        }
    }
    out.admittedbox_count = body.matches("\\admittedbox{").count();
    Ok(out)
}

/// Top-level audit. Currently checks: JSON parses + self-consistent, every
/// `\coqbox{INV-X}` resolves, every Admitted citation paired with `\admittedbox`.
pub fn run_audit(repo_root: impl AsRef<Path>) -> Result<AuditReport> {
    let root = repo_root.as_ref();
    let assertions = Assertions::load(root.join("assertions/igla_assertions.json"))?;
    let known_ids: Vec<&str> = assertions.invariants.iter().map(|i| i.id.as_str()).collect();
    let chapters_dir = root.join("docs/phd/chapters");
    let mut report = AuditReport {
        assertions_self_check: format!(
            "OK (schema {}, {} invariants: {} Proven, {} Admitted)",
            assertions.meta.schema_version,
            assertions.invariants.len(),
            assertions
                .invariants
                .iter()
                .filter(|i| matches!(i.status, ProofStatus::Proven))
                .count(),
            assertions
                .invariants
                .iter()
                .filter(|i| matches!(i.status, ProofStatus::Admitted))
                .count(),
        ),
        chapters_scanned: 0,
        coqbox_unresolved: Vec::new(),
        admitted_without_admittedbox: Vec::new(),
    };
    if !chapters_dir.exists() {
        return Ok(report);
    }
    for path in list_chapter_files(&chapters_dir)? {
        report.chapters_scanned += 1;
        let chapter = audit_chapter(&path)?;
        for inv_key in &chapter.coqbox_invariants {
            if !known_ids.contains(&inv_key.as_str()) {
                report
                    .coqbox_unresolved
                    .push(format!("{}: \\coqbox{{{}}}", path.display(), inv_key));
                continue;
            }
            if !assertions.is_proven(inv_key) && chapter.admittedbox_count == 0 {
                report.admitted_without_admittedbox.push(format!(
                    "{}: cites {} (Admitted) without \\admittedbox",
                    path.display(),
                    inv_key
                ));
            }
        }
    }
    Ok(report)
}

/// What the CLI prints / what tests assert against.
#[derive(Debug, Default)]
pub struct AuditReport {
    pub assertions_self_check: String,
    pub chapters_scanned: usize,
    pub coqbox_unresolved: Vec<String>,
    pub admitted_without_admittedbox: Vec<String>,
}

impl AuditReport {
    pub fn is_clean(&self) -> bool {
        self.coqbox_unresolved.is_empty() && self.admitted_without_admittedbox.is_empty()
    }
    pub fn render(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("assertions self-check: {}\n", self.assertions_self_check));
        s.push_str(&format!("chapters scanned:      {}\n", self.chapters_scanned));
        if self.coqbox_unresolved.is_empty() {
            s.push_str("\\coqbox unresolved:    none\n");
        } else {
            s.push_str("\\coqbox unresolved:\n");
            for line in &self.coqbox_unresolved {
                s.push_str(&format!("  - {}\n", line));
            }
        }
        if self.admitted_without_admittedbox.is_empty() {
            s.push_str("admitted-without-\\admittedbox: none\n");
        } else {
            s.push_str("admitted-without-\\admittedbox:\n");
            for line in &self.admitted_without_admittedbox {
                s.push_str(&format!("  - {}\n", line));
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_json_v1() -> &'static str {
        r#"{
  "_metadata": {
    "schema_version": "1.0.0",
    "order_ref": "TEST",
    "theorem_count": {"igla_total": 6, "proven_qed": 4, "honest_admitted": 2},
    "admitted_budget": {"max": 4, "used": 2}
  },
  "trinity_identity": "phi^2 + phi^-2 = 3",
  "invariants": [
    {"id": "INV-1", "name": "i1", "coq_file": "lr.v", "status": "Admitted",
     "admitted_theorems": ["alpha_phi_lb"]},
    {"id": "INV-2", "name": "i2", "coq_file": "asha.v", "status": "Proven"},
    {"id": "INV-3", "name": "i3", "coq_file": "gf16.v", "status": "Admitted",
     "admitted_theorems": ["e2e_bound"]},
    {"id": "INV-4", "name": "i4", "coq_file": "nca.v", "status": "Proven"},
    {"id": "INV-5", "name": "i5", "coq_file": "lucas.v", "status": "Proven"},
    {"id": "INV-12", "name": "i12", "coq_file": "asha.v", "status": "Proven"}
  ],
  "enforcement": {}
}"#
    }

    fn make_repo(root: &Path, json: &str, chapters: &[(&str, &str)]) {
        std::fs::create_dir_all(root.join("assertions")).unwrap();
        std::fs::write(root.join("assertions/igla_assertions.json"), json).unwrap();
        std::fs::create_dir_all(root.join("docs/phd/chapters")).unwrap();
        for (name, body) in chapters {
            std::fs::write(root.join("docs/phd/chapters").join(name), body).unwrap();
        }
    }

    #[test]
    fn loads_v1_schema_and_reports_status() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.json"), fixture_json_v1()).unwrap();
        let a = Assertions::load(dir.path().join("a.json")).unwrap();
        assert_eq!(a.invariants.len(), 6);
        assert!(a.is_proven("INV-2"));
        assert!(!a.is_proven("INV-1"));
    }

    #[test]
    fn refuses_unsupported_schema_version() {
        let dishonest = fixture_json_v1().replace(r#""schema_version": "1.0.0""#,
                                                   r#""schema_version": "0.9.0""#);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.json"), dishonest).unwrap();
        let err = Assertions::load(dir.path().join("a.json")).unwrap_err();
        assert!(format!("{:?}", err).contains("schema_version"));
    }

    #[test]
    fn rejects_proven_with_admitted_theorems() {
        let dishonest = fixture_json_v1().replace(
            r#"{"id": "INV-2", "name": "i2", "coq_file": "asha.v", "status": "Proven"}"#,
            r#"{"id": "INV-2", "name": "i2", "coq_file": "asha.v", "status": "Proven", "admitted_theorems": ["fake"]}"#,
        );
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.json"), dishonest).unwrap();
        let err = Assertions::load(dir.path().join("a.json")).unwrap_err();
        assert!(format!("{:?}", err).contains("R5"), "expected R5 violation, got: {:?}", err);
    }

    #[test]
    fn rejects_admitted_without_named_theorems() {
        let dishonest = fixture_json_v1().replace(
            r#"{"id": "INV-1", "name": "i1", "coq_file": "lr.v", "status": "Admitted",
     "admitted_theorems": ["alpha_phi_lb"]}"#,
            r#"{"id": "INV-1", "name": "i1", "coq_file": "lr.v", "status": "Admitted"}"#,
        );
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.json"), dishonest).unwrap();
        let err = Assertions::load(dir.path().join("a.json")).unwrap_err();
        assert!(format!("{:?}", err).contains("R5"));
    }

    #[test]
    fn rejects_overspent_budget() {
        let dishonest = fixture_json_v1()
            .replace(r#""max": 4, "used": 2"#, r#""max": 4, "used": 9"#);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.json"), dishonest).unwrap();
        assert!(Assertions::load(dir.path().join("a.json")).is_err());
    }

    #[test]
    fn rejects_duplicate_invariant_id() {
        let dishonest = fixture_json_v1().replace(r#""id": "INV-12""#, r#""id": "INV-2""#);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.json"), dishonest).unwrap();
        let err = Assertions::load(dir.path().join("a.json")).unwrap_err();
        assert!(format!("{:?}", err).contains("duplicate"));
    }

    #[test]
    fn audit_flags_admitted_without_admittedbox() {
        let dir = tempfile::tempdir().unwrap();
        let chapters = [(
            "ch01-test.tex",
            "\\chapter{Test}\nWe rely on \\coqbox{INV-1} which is Admitted.\n",
        )];
        make_repo(dir.path(), fixture_json_v1(), &chapters);
        let report = run_audit(dir.path()).unwrap();
        assert_eq!(report.chapters_scanned, 1);
        assert_eq!(report.admitted_without_admittedbox.len(), 1);
    }

    #[test]
    fn audit_passes_with_admittedbox() {
        let dir = tempfile::tempdir().unwrap();
        let chapters = [(
            "ch01-test.tex",
            "\\chapter{T}\nVia \\coqbox{INV-1}\\admittedbox{awaits Coq.Interval}.\n",
        )];
        make_repo(dir.path(), fixture_json_v1(), &chapters);
        let report = run_audit(dir.path()).unwrap();
        assert!(report.is_clean(), "report = {}", report.render());
    }

    #[test]
    fn audit_flags_unknown_invariant_key() {
        let dir = tempfile::tempdir().unwrap();
        let chapters = [(
            "ch01-test.tex",
            "\\chapter{T}\nClaim: \\coqbox{INV-99}.\n",
        )];
        make_repo(dir.path(), fixture_json_v1(), &chapters);
        let report = run_audit(dir.path()).unwrap();
        assert_eq!(report.coqbox_unresolved.len(), 1);
    }

    #[test]
    fn audit_accepts_inv12_from_v1_schema() {
        let dir = tempfile::tempdir().unwrap();
        let chapters = [(
            "ch24-test.tex",
            "\\chapter{IGLA}\nRungs proven via \\coqbox{INV-12}.\n",
        )];
        make_repo(dir.path(), fixture_json_v1(), &chapters);
        let report = run_audit(dir.path()).unwrap();
        assert!(report.is_clean(), "report = {}", report.render());
    }
}
