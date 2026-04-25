//! Citation primitives — every numeric constant in the monograph must use one.
//!
//! The two link types correspond to the two sanctioned evidence paths under
//! mission rule **R4**:
//!
//! - [`CodataLink`] — empirical reference (CODATA 2022, PDG, NIST, named experiment).
//! - [`CoqLink`]    — formal reference (`trinity-clara/proofs/igla/<file>.v` theorem).
//!
//! A constant lacking both is a Popper's-razor violation (#39 / R6) and the
//! `audit` subcommand will reject it.

use serde::{Deserialize, Serialize};

/// Empirical citation — CODATA 2022, PDG, NIST, or named experiment paper.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodataLink {
    /// Constant name, e.g. `"alpha_inv"` for the inverse fine-structure constant.
    pub name: String,
    /// Reference value (e.g. CODATA 2022 best estimate).
    pub value: f64,
    /// Standard uncertainty (1σ) from the source.
    pub uncertainty: f64,
    /// Source identifier — `"CODATA-2022"`, `"PDG-2024"`, `"Coldea-2010"`, etc.
    pub source: String,
    /// LaTeX macro key — `\codatabox{<key>}` resolves through this.
    pub latex_key: String,
}

impl CodataLink {
    /// Verify that `actual` is consistent with the cited value within `k_sigma·σ`.
    pub fn within_sigma(&self, actual: f64, k_sigma: f64) -> bool {
        (actual - self.value).abs() <= k_sigma * self.uncertainty.max(f64::EPSILON)
    }

    /// Render as a LaTeX macro line for inclusion in a generated `cite-table.tex`.
    pub fn to_latex(&self) -> String {
        format!(
            "\\newcommand{{\\codata{}}}{{{:.6} \\pm {:.1e} \\;\\text{{[{}]}}}}",
            self.latex_key, self.value, self.uncertainty, self.source
        )
    }
}

/// Formal citation — a theorem in a `.v` file under `trinity-clara/proofs/`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoqLink {
    /// Invariant identifier as listed in `assertions/igla_assertions.json`,
    /// e.g. `"INV-2"` or `"INV-5"`.
    pub invariant: String,
    /// Theorem name inside the `.v` file (e.g. `"champion_survives_pruning"`).
    pub theorem: String,
    /// Path relative to repo root, e.g. `"trinity-clara/proofs/igla/igla_asha_bound.v"`.
    pub proof_file: String,
    /// `Proven` (Qed.) or `Admitted` — must match the JSON verbatim per mission rule R5.
    pub status: ProofStatus,
    /// LaTeX macro key — `\coqbox{<key>}` resolves through this.
    pub latex_key: String,
}

/// Honest proof status, propagated unchanged from `assertions/igla_assertions.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStatus {
    /// All proof obligations closed with `Qed.`
    Proven,
    /// At least one proof obligation closed with `Admitted.` — must be paired
    /// with `\admittedbox{...}` in any chapter that cites it (rule R5).
    Admitted,
}

impl CoqLink {
    /// True iff this link refers to a fully-proved theorem.
    pub fn is_proven(&self) -> bool {
        matches!(self.status, ProofStatus::Proven)
    }

    /// Render as a LaTeX command for inclusion in a generated `coq-table.tex`.
    pub fn to_latex(&self) -> String {
        let badge = match self.status {
            ProofStatus::Proven => "\\textsc{Qed}",
            ProofStatus::Admitted => "\\textsc{Admitted}",
        };
        format!(
            "\\newcommand{{\\coq{}}}{{[{}: \\texttt{{{}}}, {}]}}",
            self.latex_key, self.invariant, self.theorem, badge
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codata_within_sigma_accepts_consistent_value() {
        let link = CodataLink {
            name: "alpha_inv".into(),
            value: 137.035_999_177,
            uncertainty: 2.1e-8 * 137.0,
            source: "CODATA-2022".into(),
            latex_key: "alphaInv".into(),
        };
        assert!(link.within_sigma(137.035_999_180, 3.0));
        assert!(!link.within_sigma(137.5, 3.0));
    }

    #[test]
    fn coq_link_status_is_honest() {
        let proven = CoqLink {
            invariant: "INV-2".into(),
            theorem: "champion_survives_pruning".into(),
            proof_file: "trinity-clara/proofs/igla/igla_asha_bound.v".into(),
            status: ProofStatus::Proven,
            latex_key: "invTwo".into(),
        };
        let admitted = CoqLink {
            status: ProofStatus::Admitted,
            invariant: "INV-1".into(),
            theorem: "alpha_phi_lb".into(),
            proof_file: "trinity-clara/proofs/igla/lr_convergence.v".into(),
            latex_key: "invOne".into(),
        };
        assert!(proven.is_proven());
        assert!(!admitted.is_proven());
        // R5: Admitted must surface in LaTeX rendering.
        assert!(admitted.to_latex().contains("Admitted"));
    }
}
