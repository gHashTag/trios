//! SILVER-RING-DR-00 — Core types for trios-doctor
//! No business logic. No I/O. Pure data definitions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceDiagnosis {
    pub workspace: String,
    pub crate_count: usize,
    pub checks: Vec<WorkspaceCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub failed_crates: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    Green,
    Yellow,
    Red,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Green => write!(f, "✅ Green"),
            CheckStatus::Yellow => write!(f, "⚠️ Yellow"),
            CheckStatus::Red => write!(f, "❌ Red"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_status_equality() {
        assert_eq!(CheckStatus::Green, CheckStatus::Green);
        assert_ne!(CheckStatus::Green, CheckStatus::Red);
        assert_ne!(CheckStatus::Yellow, CheckStatus::Red);
    }

    #[test]
    fn workspace_diagnosis_serde_roundtrip() {
        let diag = WorkspaceDiagnosis {
            workspace: "/fake".into(),
            crate_count: 5,
            checks: vec![WorkspaceCheck {
                name: "check".into(),
                status: CheckStatus::Green,
                message: "ok".into(),
                failed_crates: vec!["foo".into()],
            }],
        };
        let json = serde_json::to_string(&diag).unwrap();
        let back: WorkspaceDiagnosis = serde_json::from_str(&json).unwrap();
        assert_eq!(back.workspace, "/fake");
        assert_eq!(back.crate_count, 5);
        assert_eq!(back.checks.len(), 1);
    }

    #[test]
    fn check_status_display() {
        assert_eq!(format!("{}", CheckStatus::Green), "✅ Green");
        assert_eq!(format!("{}", CheckStatus::Red), "❌ Red");
    }
}
