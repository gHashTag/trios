//! L-f4: Victory Gate for Gate-final (BPB < 1.50 on 3 seeds)

use std::fs::File;
use std::io::{BufRead, BufReader};

const SEED_RESULTS: &str = "assertions/seed_results.jsonl";
const BPB_THRESH: f64 = 1.50;
const BASELINE_MU: f64 = 1.55;
const ALPHA: f64 = 0.01;

#[derive(Debug, Clone)]
pub struct VictoryRecord {
    pub achieved: bool,
    pub min_bpb: f64,
    pub mean_bpb: f64,
    pub p_value: f64,
    pub failed_seeds: Vec<u64>,
}

#[derive(Debug, Clone)]
struct SeedResult {
    seed: u64,
    bpb: f64,
}

fn read_last_3() -> Vec<SeedResult> {
    let file = match File::open(SEED_RESULTS) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = vec![];
    for line in reader.lines().map_while(Result::ok) {
        if !line.is_empty() {
            lines.push(line);
        }
    }
    let start = if lines.len() >= 3 { lines.len() - 3 } else { 0 };
    lines[start..].iter().filter_map(|l| parse_jsonl(l)).collect()
}

fn parse_jsonl(line: &str) -> Option<SeedResult> {
    let seed = line.split(r#""seed":"#).nth(1)?.split('"').next()?.parse().ok()?;
    let bpb = line.split(r#""bpb":"#).nth(1)?.split('"').next()?.parse().ok()?;
    Some(SeedResult { seed, bpb })
}

fn welch_t(samples: &[f64]) -> f64 {
    if samples.len() < 2 {
        return 1.0;
    }
    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let var = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let t = (mean - BASELINE_MU) / (var / n).sqrt();
    let abs_t = t.abs();
    if abs_t > 3.0 { 0.001 } else if abs_t > 2.5 { 0.01 } else { 0.05 }
}

pub fn check_victory() -> VictoryRecord {
    let tail = read_last_3();
    if tail.len() < 3 {
        return VictoryRecord { achieved: false, min_bpb: f64::NAN, mean_bpb: f64::NAN, p_value: 1.0, failed_seeds: vec![] };
    }
    let bpbs: Vec<f64> = tail.iter().map(|r| r.bpb).collect();
    let min_bpb = bpbs.iter().cloned().fold(f64::INFINITY, f64::min);
    let mean_bpb = bpbs.iter().sum::<f64>() / bpbs.len() as f64;
    let failed: Vec<u64> = tail.iter().filter(|r| r.bpb >= BPB_THRESH).map(|r| r.seed).collect();
    let p = welch_t(&bpbs);
    VictoryRecord { achieved: failed.is_empty() && p < ALPHA && min_bpb < BPB_THRESH, min_bpb, mean_bpb, p_value: p, failed_seeds: failed }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_victory_struct() {
        let _ = check_victory();
    }
}
