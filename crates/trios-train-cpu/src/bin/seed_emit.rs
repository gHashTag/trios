//! L-f3: Seed Results Emitter for Gate-final
//!
//! Appends 3 rows to assertions/seed_results.jsonl for seeds {42, 43, 44}.
//! Each row records: seed, step, bpb, sha, timestamp.
//!
//! Refs: trios#143 Gate-final DRAFT §2, L-f3

use std::fs::OpenOptions;
use std::io::Write;

const SEED_RESULTS_PATH: &str = "assertions/seed_results.jsonl";

#[derive(Debug, Clone)]
pub struct SeedResultRow {
    pub seed: u64,
    pub step: usize,
    pub bpb: f32,
    pub sha: String,
    pub timestamp: String,
}

impl SeedResultRow {
    pub fn to_jsonl(&self) -> String {
        format!(
            r#"{{"seed":{},"step":{},"bpb":{},"sha":"{}","timestamp":"{}"}}"#,
            self.seed, self.step, self.bpb, self.sha, self.timestamp
        )
    }
}

pub fn append_seed_result(row: &SeedResultRow) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(SEED_RESULTS_PATH)?;
    writeln!(file, "{}", row.to_jsonl())?;
    Ok(())
}

/// Emit 3 rows for seeds {42, 43, 44} (Gate-final requirement)
pub fn emit_gate_final_seeds(
    step: usize,
    bpbs: [f32; 3], // [seed42, seed43, seed44]
    sha: &str,
) -> std::io::Result<()> {
    let seeds = [42, 43, 44];
    let timestamp = chrono::Utc::now().to_rfc3339();

    for (i, &seed) in seeds.iter().enumerate() {
        let row = SeedResultRow {
            seed,
            step,
            bpb: bpbs[i],
            sha: sha.to_string(),
            timestamp: timestamp.clone(),
        };
        append_seed_result(&row)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_seed_result_to_jsonl() {
        let row = SeedResultRow {
            seed: 43,
            step: 54000,
            bpb: 1.85,
            sha: "abc123".to_string(),
            timestamp: "2026-04-26T10:00:00Z".to_string(),
        };
        let jsonl = row.to_jsonl();
        assert!(jsonl.contains("\"seed\":43"));
        assert!(jsonl.contains("\"bpb\":1.85"));
    }

    #[test]
    fn test_emit_gate_final_seeds_structure() {
        // Just test that the function would produce correct structure
        // without actually writing to disk
        let seeds = [42, 43, 44];
        let bpbs = [1.48, 1.49, 1.47];
        for (i, &seed) in seeds.iter().enumerate() {
            let row = SeedResultRow {
                seed,
                step: 81000,
                bpb: bpbs[i],
                sha: "test".to_string(),
                timestamp: "2026-04-26T10:00:00Z".to_string(),
            };
            let jsonl = row.to_jsonl();
            assert!(jsonl.contains(&format!("\"seed\":{}", seed)));
            assert!(jsonl.contains(&format!("\"bpb\":{}", bpbs[i])));
        }
    }
}

fn main() {
    // Seed emitter CLI
    // Usage: seed_emit <step> <bpb42> <bpb43> <bpb44> <sha>
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        eprintln!("Usage: {} <step> <bpb42> <bpb43> <bpb44> <sha>", args[0]);
        std::process::exit(1);
    }

    let step: usize = args[1].parse().expect("step must be usize");
    let bpb42: f32 = args[2].parse().expect("bpb42 must be f32");
    let bpb43: f32 = args[3].parse().expect("bpb43 must be f32");
    let bpb44: f32 = args[4].parse().expect("bpb44 must be f32");
    let sha = &args[5];

    let bpbs = [bpb42, bpb43, bpb44];
    if let Err(e) = emit_gate_final_seeds(step, bpbs, sha) {
        eprintln!("Error emitting seed results: {}", e);
        std::process::exit(1);
    }

    println!("Emitted 3 seed results to {}", SEED_RESULTS_PATH);
}
