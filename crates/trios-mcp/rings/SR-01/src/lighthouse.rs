//! Lighthouse CLI execution via std::process::Command
//!
//! Executes `node lighthouse` with appropriate arguments and parses JSON output.

use anyhow::{Context, Result};
use serde_json::Value;
use tracing::{debug, warn};

use crate::{AuditCategory, Metadata};

/// Lighthouse execution configuration
#[derive(Debug, Clone)]
pub struct LighthouseConfig {
    pub node_path: String,
    pub timeout_secs: u64,
}

impl Default for LighthouseConfig {
    fn default() -> Self {
        Self {
            node_path: "node".to_string(),
            timeout_secs: 60,
        }
    }
}

/// Run Lighthouse via node CLI
pub fn run_lighthouse(
    url: &str,
    categories: &[AuditCategory],
) -> Result<Value> {
    let categories_str = categories
        .iter()
        .map(|c| serde_json::to_string(c).unwrap())
        .collect::<Vec<_>>()
        .join(",");

    let args = vec![
        "lighthouse",
        url,
        "--output=json",
        "--only-categories",
        &categories_str,
        "--quiet",
    ];

    debug!("Executing: node lighthouse {} --only-categories {}", url, categories_str);

    let output = std::process::Command::new("node")
        .args(&args)
        .output()
        .context("Failed to execute Lighthouse. Is 'lighthouse' installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Lighthouse failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let report: Value = serde_json::from_str(&stdout)
        .context("Failed to parse Lighthouse JSON output")?;

    Ok(report)
}

/// Extract metadata from Lighthouse report
pub fn extract_metadata(report: &Value) -> Metadata {
    Metadata {
        url: report.get("finalUrl")
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        device: "desktop".to_string(),
        lighthouse_version: report.get("lighthouseVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
    }
}

/// Check if Lighthouse is available
pub fn check_lighthouse_available() -> bool {
    let result = std::process::Command::new("node")
        .args(&["lighthouse", "--version"])
        .output();

    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lighthouse_config_default() {
        let config = LighthouseConfig::default();
        assert_eq!(config.node_path, "node");
        assert_eq!(config.timeout_secs, 60);
    }

    #[test]
    fn test_check_lighthouse_available() {
        // This test just ensures the function doesn't panic
        let available = check_lighthouse_available();
        debug!("Lighthouse available: {}", available);
    }
}
