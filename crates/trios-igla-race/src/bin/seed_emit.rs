//! L-f3: Seed Emission for Gate-final Victory
//!
//! Appends 3 rows on seeds {42, 43, 44} to assertions/seed_results.jsonl
//! with schema validation. Each row contains: seed, step, bpb, sha, timestamp.
//!
//! ## Pre-registration
//!
//! Refs: trios#143 Gate-final DRAFT §9
//!
//! ## Schema
//!
//! | Field      | Type      | Description                                  |
//! |------------|-----------|----------------------------------------------|
//! | `seed`     | `u64`    | Seed value ∈ {42, 43, 44}              |
//! | `step`     | `usize`   | Training step (≥ 4000 for victory)      |
//! | `bpb`      | `f64`    | Bits per byte (must be < 20.0)           |
//! | `sha`      | `string`   | Git commit SHA (full 40-char)          |
//! | `timestamp`| `string`   | ISO 8601 UTC                          |
//!
//! ## Validation
//!
//! - `seed` ∈ {42, 43, 44} (Gate-final only seeds)
//! - `bpb` > 0 && `bpb` < 20.0 (L-METRIC physical range)
//! - `step` ≥ 4000 (victory threshold)
//! - `sha` is valid 40-char hex string
//! - `timestamp` is ISO 8601 format
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p trios-igla-race --bin seed_emit -- --seed 43 --bpb 1.2345 --step 54000 --sha a1b2c3d
//! ```
//!
//! ## Owner
//!
//! Lane: L-f3
//! Agent: igla-l-f3-ledger
//! INV: (schema only - no invariant dependencies)
//! Hours: 1

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufWriter, Write};

const SEED_RESULTS_PATH: &str = "assertions/seed_results.jsonl";

/// Minimum step for Gate-final victory check (DRAFT §2)
const VICTORY_MIN_STEP: usize = 4000;

/// Minimum BPB for L-METRIC validation (not real, floor)
const MIN_BPB: f64 = 0.1;

/// Maximum BPB for L-METRIC validation (physical range)
const MAX_BPB: f64 = 20.0;

#[derive(Debug, Clone)]
struct SeedRow {
    pub seed: u64,
    pub step: usize,
    pub bpb: f64,
    pub sha: String,
    pub timestamp: String,
}

/// Validate a SeedRow against schema constraints.
fn validate_row(row: &SeedRow) -> Result<(), String> {
    // Validate seed ∈ {42, 43, 44}
    if ![42u64, 43, 44].contains(&row.seed) {
        return Err(format!("seed {} not in {{42, 43, 44}}", row.seed));
    }

    // Validate BPB physical range
    if row.bpb < MIN_BPB || row.bpb >= MAX_BPB {
        return Err(format!("bpb {:.4} outside physical range [{:.1}, {}]", row.bpb, MIN_BPB, MAX_BPB));
    }

    // Validate step ≥ victory threshold
    if row.step < VICTORY_MIN_STEP {
        return Err(format!("step {} < VICTORY_MIN_STEP {}", row.step, VICTORY_MIN_STEP));
    }

    // Validate SHA is 40-char hex
    if row.sha.len() != 40 {
        return Err(format!("sha '{}' is not 40 characters", row.sha));
    }
    // Simple hex validation
    if !row.sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("sha '{}' contains non-hex characters", row.sha));
    }

    // Validate timestamp is ISO 8601 format (simplified check)
    if row.timestamp.is_empty() {
        return Err("timestamp cannot be empty".to_string());
    }

    Ok(())
}

/// Read existing seed_results.jsonl and return all rows as Vec<SeedRow>.
fn read_existing_rows() -> Vec<SeedRow> {
    let file = match File::open(SEED_RESULTS_PATH) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let reader = BufReader::new(file);
    let mut rows = vec![];

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse JSONL format: {"seed":42,"step":54000,"bpb":2.23,...}
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(map) = value.as_object() {
                let seed = map.get("seed")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let step = map.get("step")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let bpb = map.get("bpb")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let sha = map.get("sha")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|_| "".to_string());
                let timestamp = map.get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|_| "".to_string());

                if seed > 0 {
                    rows.push(SeedRow {
                        seed,
                        step,
                        bpb,
                        sha,
                        timestamp,
                    });
                }
            }
        }
    }

    rows
}

/// Append new rows to seed_results.jsonl.
fn append_rows(rows: &[SeedRow]) -> Result<(), Box<dyn std::error::Error>> {
    // Open file in append mode
    let mut file = File::options()
        .write(true)
        .append(true)
        .create(SEED_RESULTS_PATH)?;

    let mut writer = BufWriter::new(&file);

    // Write each new row as JSONL
    for row in rows {
        let row_json = serde_json::json!({
            "seed": row.seed,
            "step": row.step,
            "bpb": row.bpb,
            "sha": row.sha,
            "timestamp": row.timestamp,
        });

        writeln!(writer, "{}", row_json)?;
    }

    Ok(())
}

fn main() {
    // Parse CLI arguments
    let args: Vec<String> = std::env::args().collect();
    let mut seeds: Vec<u64> = vec![];
    let mut bpbs: Vec<f64> = vec![];
    let mut steps: Vec<usize> = vec![];
    let mut shas: Vec<String> = vec![];

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                i += 1;
                if i < args.len() {
                    if let Ok(s) = args[i].parse::<u64>() {
                        seeds.push(s);
                    }
                }
            }
            "--bpb" => {
                i += 1;
                if i < args.len() {
                    if let Ok(b) = args[i].parse::<f64>() {
                        bpbs.push(b);
                    }
                }
            }
            "--step" => {
                i += 1;
                if i < args.len() {
                    if let Ok(s) = args[i].parse::<usize>() {
                        steps.push(s);
                    }
                }
            }
            "--sha" => {
                i += 1;
                if i < args.len() {
                    shas.push(args[i].clone());
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    // Validate required arguments
    if seeds.is_empty() || seeds.len() != 3 {
        eprintln!("Usage: --seed <S1> --seed <S2> --seed <S3> --bpb <B1> --bpb <B2> --bpb <B3> --step <STEP1> --step <STEP2> --step <STEP3> --sha <SHA1> [--sha <SHA2> [--sha <SHA3>]");
        eprintln!("  Seeds must be exactly 3 values from {{42, 43, 44}}");
        eprintln!("  BPBs, steps, SHAs must match seeds count");
        std::process::exit(1);
    }

    if bpbs.len() != 3 || steps.len() != 3 {
        eprintln!("Error: BPBs, steps, SHAs must match seeds count (3 each)");
        std::process::exit(1);
    }

    // Use git SHA if not provided
    if shas.is_empty() {
        let output = std::process::Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .output(std::process::Stdio::piped())
            .spawn();
        let result = output.wait_with_output();
        if let Ok(sha_output) = result {
            if let Some(sha_line) = sha_output.lines().next() {
                shas.push(sha_line.trim().to_string());
            }
        }
        if shas.len() < 3 {
            eprintln!("Warning: Could not auto-detect 3 SHAs from git");
        }
    }

    if shas.len() < 3 {
        eprintln!("Error: Need 3 SHA values (or auto-detect from git)");
        std::process::exit(1);
    }

    // Validate each set of arguments
    for i in 0..3 {
        validate_row(&SeedRow {
            seed: seeds[i],
            step: steps[i],
            bpb: bpbs[i],
            sha: shas[i].clone(),
            timestamp: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
        }).expect(&format!("Invalid row {} (seed={}, step={}, bpb={})", i + 1, seeds[i], steps[i], bpbs[i]));
    }

    // Read existing rows
    let existing_rows = read_existing_rows();

    // Append new rows
    let new_rows: Vec<SeedRow> = vec![
        SeedRow {
            seed: seeds[0],
            step: steps[0],
            bpb: bpbs[0],
            sha: shas[0].clone(),
            timestamp: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
        },
        SeedRow {
            seed: seeds[1],
            step: steps[1],
            bpb: bpbs[1],
            sha: shas[1].clone(),
            timestamp: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
        },
        SeedRow {
            seed: seeds[2],
            step: steps[2],
            bpb: bpbs[2],
            sha: shas[2].clone(),
            timestamp: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
        },
    ];

    // Append to file
    match append_rows(&new_rows) {
        Ok(()) => {
            println!("SUCCESS: Appended 3 rows to {}", SEED_RESULTS_PATH);
            for row in &new_rows {
                println!("  seed={} step={} bpb={:.4} sha={}", row.seed, row.step, row.bpb, row.sha);
            }
        }
        Err(e) => {
            eprintln!("ERROR: Failed to append rows: {}", e);
            std::process::exit(1);
        }
    }
}
