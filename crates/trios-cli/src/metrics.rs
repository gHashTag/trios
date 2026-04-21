//! Metrics validation and parsing for BPB, model size, training time

use anyhow::{bail, Result};

/// Validate BPB value (bits per byte)
///
/// BPB is typically 2-8 for good language models.
/// Values outside [0, 10] are suspicious.
pub fn validate_bpb(bpb: f64) -> Result<()> {
    if bpb.is_nan() || bpb.is_infinite() {
        bail!("BPB must be finite, got: {}", bpb);
    }
    if bpb < 0.0 || bpb > 15.0 {
        bail!("BPB out of reasonable range [0, 15]: {}", bpb);
    }
    Ok(())
}

/// Validate model size in parameters
pub fn validate_param_count(params: u64) -> Result<()> {
    if params == 0 {
        bail!("Model size cannot be zero");
    }
    if params > 1_000_000_000 {
        bail!("Model size too large: {} (max 1B params)", params);
    }
    Ok(())
}

/// Validate training time in seconds
pub fn validate_time(seconds: f64) -> Result<()> {
    if seconds < 0.0 || seconds.is_nan() || seconds.is_infinite() {
        bail!("Invalid training time: {}", seconds);
    }
    Ok(())
}

/// Parse BPB from stdout using regex patterns
///
/// Matches patterns like:
/// - val_bpb=6.5609
/// - Val BPB: 6.5609
/// - val_bpb: 6.56
pub fn parse_bpb_from_output(output: &str) -> Result<f64> {
    let patterns = [
        r"val_bpb[=:]\s*([0-9.]+)",
        r"val_bpb[=][:]\s*([0-9.]+)",
        r"Val BPB[=:]\s*([0-9.]+)",
    ];

    for pattern in &patterns {
        if let Some(captures) = regex::Regex::new(pattern).ok().and_then(|re| re.captures(output)) {
            if let Some(bpb_str) = captures.get(1) {
                let bpb = bpb_str.as_str().parse::<f64>()
                    .context("Failed to parse BPB as float")?;
                validate_bpb(bpb)?;
                return Ok(bpb);
            }
        }
    }

    bail!("Could not parse BPB from output. Tried patterns: {:?}", patterns)
}

/// Parse model parameters from output
///
/// Matches patterns like:
/// - params=1234567
/// - Model: 1.2M params
pub fn parse_params_from_output(output: &str) -> Result<u64> {
    let patterns = [
        r"params[=:]\s*([0-9]+)",
        r"params[=:]s*([0-9]+)",
        r"model.*([0-9.]+)M.*params",
    ];

    for pattern in &patterns {
        if let Some(captures) = regex::Regex::new(pattern).ok().and_then(|re| re.captures(output)) {
            if let Some(params_str) = captures.get(1) {
                let params = params_str.as_str().parse::<u64>()
                    .context("Failed to parse params as integer")?;
                validate_param_count(params)?;
                return Ok(params);
            }
        }
    }

    // Try to parse "M" suffix (e.g., "1.2M")
    if let Some(caps) = regex::Regex::new(r"([0-9.]+)M").ok().and_then(|re| re.captures(output)) {
        if let Some(m_str) = caps.get(1) {
            let m: f64 = m_str.as_str().parse()?;
            return Ok((m * 1_000_000.0) as u64);
        }
    }

    bail!("Could not parse params from output")
}

/// Parse training time from output
///
/// Matches patterns like:
/// - elapsed=123.45s
/// - time: 123.45s
/// - Training time: 123.45 seconds
pub fn parse_time_from_output(output: &str) -> Result<f64> {
    let patterns = [
        r"elapsed[=:]\s*([0-9.]+)s",
        r"time[=:]\s*([0-9.]+)s",
        r"Training time[=:]\s*([0-9.]+).*seconds",
    ];

    for pattern in &patterns {
        if let Some(captures) = regex::Regex::new(pattern).ok().and_then(|re| re.captures(output)) {
            if let Some(time_str) = captures.get(1) {
                let time = time_str.as_str().parse::<f64>()
                    .context("Failed to parse time as float")?;
                validate_time(time)?;
                return Ok(time);
            }
        }
    }

    bail!("Could not parse training time from output")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_bpb() {
        assert!(validate_bpb(6.5).is_ok());
        assert!(validate_bpb(0.0).is_ok());
        assert!(validate_bpb(15.0).is_ok());
        assert!(validate_bpb(-1.0).is_err());
        assert!(validate_bpb(20.0).is_err());
        assert!(validate_bpb(f64::NAN).is_err());
    }

    #[test]
    fn test_parse_bpb() {
        let output = "Training...\nval_bpb=6.5609\nDone";
        assert_eq!(parse_bpb_from_output(output).unwrap(), 6.5609);

        let output2 = "Val BPB: 7.23\nTraining complete";
        assert_eq!(parse_bpb_from_output(output2).unwrap(), 7.23);
    }
}
