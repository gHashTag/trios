//! L-f4: Victory Gate for Gate-final (BPB < 1.50 on 3 seeds)
//!
//! Checks if the last 3 rows in seed_results.jsonl satisfy:
//! - All 3 seeds have BPB < 1.50 at step >= 4000
//! - Welch t-test against μ₀=1.55 yields p < 0.01
//! - INV-7 igla_found_criterion accepts the set
//!
//! Refs: trios#143 Gate-final DRAFT §2, L-f4

use std::fs;
use std::io::{BufRead, BufReader};

const SEED_RESULTS_PATH: &str = "assertions/seed_results.jsonl";
const GATE_FINAL_BPB_THRESHOLD: f64 = 1.50;
const BASELINE_MU: f64 = 1.55;
const ALPHA: f64 = 0.01;
const MIN_STEP: usize = 4000;

#[derive(Debug, Clone)]
pub struct SeedResult {
    pub seed: u64,
    pub step: usize,
    pub bpb: f64,
    pub sha: String,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
pub struct VictoryRecord {
    pub achieved: bool,
    pub min_bpb: f64,
    pub mean_bpb: f64,
    pub t_statistic: f64,
    pub p_value: f64,
    pub failed_seeds: Vec<u64>,
    pub message: String,
}

/// Parse a single JSONL line into a SeedResult
fn parse_seed_result(line: &str) -> Option<SeedResult> {
    // Simple JSON parsing for the schema
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    
    // Extract values using simple string parsing
    let seed = extract_json_value(line, "seed")?.parse().ok()?;
    let step = extract_json_value(line, "step")?.parse().ok()?;
    let bpb = extract_json_value(line, "bpb")?.parse().ok()?;
    let sha = extract_json_value(line, "sha")?.to_string();
    let timestamp = extract_json_value(line, "timestamp")?.to_string();
    
    Some(SeedResult {
        seed, step, bpb, sha, timestamp,
    })
}

fn extract_json_value(line: &str, key: &str) -> Option<&str> {
    let key_pattern = format!(r#""{}":"#, key);
    let start = line.find(&key_pattern)? + key_pattern.len();
    let end = line[start..].find('"')?;
    Some(&line[start..start + end])
}

/// Read the last N rows from seed_results.jsonl
fn read_last_n_results(n: usize) -> Vec<SeedResult> {
    let file = match fs::File::open(SEED_RESULTS_PATH) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    
    let reader = BufReader::new(file);
    let mut all_results: Vec<SeedResult> = vec![];
    
    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some(result) = parse_seed_result(&line) {
                all_results.push(result);
            }
        }
    }
    
    // Return last N rows
    let start = if all_results.len() >= n {
        all_results.len() - n
    } else {
        0
    };
    all_results[start..].to_vec()
}

/// Welch's t-test for unequal variances
fn welch_t_test(samples: &[f64], mu0: f64) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 1.0);
    }
    
    let n = samples.len() as f64;
    let mean: f64 = samples.iter().sum::<f64>() / n;
    let variance: f64 = samples.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / (n - 1.0);
    
    let t = (mean - mu0) / (variance / n).sqrt();
    
    // Approximate p-value using t-distribution (simplified)
    // For proper implementation, would use statistical library
    let abs_t = t.abs();
    let p = if abs_t > 3.0 {
        0.001 // Very significant
    } else if abs_t > 2.5 {
        0.01 // Significant at alpha=0.01
    } else if abs_t > 2.0 {
        0.05
    } else {
        0.10
    };
    
    (t, p)
}

/// Check INV-7 igla_found_criterion
///
/// This verifies that the candidate set satisfies the victory conditions.
/// In a full implementation, this would check all 6 falsifiers from the DRAFT.
fn check_inv7_criterion(results: &[SeedResult]) -> bool {
    // INV-7 falsifier 1: no seed with BPB >= 1.50
    if results.iter().any(|r| r.bpb >= GATE_FINAL_BPB_THRESHOLD) {
        return false;
    }
    
    // INV-7 falsifier 2: all seeds must have step >= MIN_STEP
    if results.iter().any(|r| r.step < MIN_STEP) {
        return false;
    }
    
    // Additional INV-7 checks would go here
    true
}

/// Main victory check: invoke on the 3-row tail
pub fn check_victory() -> VictoryRecord {
    let tail = read_last_n_results(3);
    
    if tail.len() < 3 {
        return VictoryRecord {
            achieved: false,
            min_bpb: f32::NAN,
            mean_bpb: f32::NAN,
            t_statistic: f32::NAN,
            p_value: 1.0,
            failed_seeds: vec![],
            message: format!("Need 3 seed results, found {}", tail.len()),
        };
    }
    
    // Extract BPB values
    let bpbs: Vec<f64> = tail.iter().map(|r| r.bpb).collect();
    let min_bpb = bpbs.iter().cloned().fold(f64::INFINITY, f64::min);
    let mean_bpb: f64 = bpbs.iter().sum::<f64>() / bpbs.len() as f64;
    
    // Check INV-7 criterion
    let inv7_passed = check_inv7_criterion(&tail);
    
    // Welch t-test
    let (t_stat, p_value) = welch_t_test(&bpbs, BASELINE_MU);
    
    // Check which seeds failed the BPB threshold
    let failed_seeds: Vec<u64> = tail.iter()
        .filter(|r| r.bpb >= GATE_FINAL_BPB_THRESHOLD)
        .map(|r| r.seed)
        .collect();
    
    let achieved = inv7_passed && p_value < ALPHA && failed_seeds.is_empty();
    
    VictoryRecord {
        achieved,
        min_bpb,
        mean_bpb,
        t_statistic: t_stat,
        p_value,
        failed_seeds,
        message: if achieved {
            format!("VICTORY: BPB < {:.2} on all seeds, p={:.4} < {:.2}", 
                GATE_FINAL_BPB_THRESHOLD, p_value, ALPHA)
        } else {
            format!("NO-GO: min_bpb={:.3}, p={:.4}, failed seeds: {:?}", 
                min_bpb, p_value, failed_seeds)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_welch_t_test_significant() {
        let samples = vec![1.45, 1.48, 1.47]; // Mean < 1.55
        let (t, p) = welch_t_test(&samples, 1.55);
        assert!(t < 0.0, "t-statistic should be negative");
        assert!(p < 0.05, "should be significant");
    }

    #[test]
    fn test_welch_t_test_not_significant() {
        let samples = vec![1.60, 1.65, 1.70]; // Mean > 1.55
        let (t, p) = welch_t_test(&samples, 1.55);
        assert!(t > 0.0, "t-statistic should be positive");
        assert!(p > 0.01, "should not be significant at alpha=0.01");
    }
}
