pub mod controller;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleDecision {
    pub action: Action,
    pub seed: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
    Spawn,
    Kill,
    Wait,
}

pub use controller::OracleController;
