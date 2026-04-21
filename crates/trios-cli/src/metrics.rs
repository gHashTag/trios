use anyhow::Result;

/// BPB/size/time validators
pub struct MetricsValidator;

impl MetricsValidator {
    pub const GATES: &'static [(&'static str, f64)] = &[
        ("G-BGH", 1.50),
        ("G-ORTH", 1.20),
        ("G-SWA", 1.15),
        ("G-STACK", 1.12),
        ("G-NEEDLE", 1.00),
    ];

    pub fn check_gate(gate: &str, bpb: f64) -> Result<bool> {
        let threshold = Self::threshold(gate)?;
        Ok(bpb <= threshold)
    }

    pub fn threshold(gate: &str) -> Result<f64> {
        Self::GATES
            .iter()
            .find(|(name, _)| *name == gate)
            .map(|(_, t)| *t)
            .ok_or_else(|| anyhow::anyhow!("Unknown gate: {gate}"))
    }

    pub fn best_gate(bpb: f64) -> Option<&'static str> {
        Self::GATES
            .iter()
            .rev()
            .find(|(_, t)| bpb <= *t)
            .map(|(name, _)| *name)
    }
}
