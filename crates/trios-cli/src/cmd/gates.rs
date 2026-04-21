//! `tri gates` — Check quality gates
//!
//! Usage:
//!   tri gates check bpab --value 5.5
//!   tri gates check all

use anyhow::{Context, Result};
use std::collections::HashMap;

use crate::db::Leaderboard;

/// Quality gate thresholds
#[expect(dead_code)]
const GATES: &[(&str, f64)] = &[
    ("bpab_target", 6.0),
    ("bpab_max", 8.0),
    ("params_max", 1_000_000.0),
    ("time_max", 3600.0),
];

/// Check specific gate
pub fn gate_check(gate: &str, value: Option<f64>) -> Result<GateStatus> {
    println!("Checking gate: {}", gate);

    match gate {
        "bpab" => check_bpab(value),
        "size" => check_size(value),
        "time" => check_time(value),
        "all" => check_all(),
        _ => anyhow::bail!("Unknown gate: {}. Use: bpab, size, time, all", gate),
    }
}

fn check_bpab(value: Option<f64>) -> Result<GateStatus> {
    let target = 6.0;
    let max = 8.0;

    let v = value.context("BPB value required for bpab gate")?;

    let status = if v <= target {
        GateStatus::Pass
    } else if v <= max {
        GateStatus::Warn
    } else {
        GateStatus::Fail
    };

    println!(
        "  BPB gate: {} (target={}, max={}) -> {:?}",
        v, target, max, status
    );

    Ok(status)
}

fn check_size(value: Option<f64>) -> Result<GateStatus> {
    let max = 1_000_000.0;

    let v = value.context("Size value required for size gate")?;

    let status = if v <= max {
        GateStatus::Pass
    } else {
        GateStatus::Fail
    };

    println!("  Size gate: {} params (max={}) -> {:?}", v, max, status);

    Ok(status)
}

fn check_time(value: Option<f64>) -> Result<GateStatus> {
    let max = 3600.0;

    let v = value.context("Time value required for time gate")?;

    let status = if v <= max {
        GateStatus::Pass
    } else {
        GateStatus::Fail
    };

    println!("  Time gate: {}s (max={}) -> {:?}", v, max, status);

    Ok(status)
}

fn check_all() -> Result<GateStatus> {
    println!("Checking all gates from leaderboard...");

    let lb = Leaderboard::open()?;
    let stats = lb.stats()?;

    let mut results: HashMap<&'static str, GateStatus> = HashMap::new();

    let bpab_status = if stats.count == 0 {
        println!("  bpab: no data (target=6.0, max=8.0)");
        GateStatus::Unknown
    } else if stats.min_bpb <= 6.0 {
        println!("  bpab: PASS (best={:.4}, target=6.0)", stats.min_bpb);
        GateStatus::Pass
    } else if stats.min_bpb <= 8.0 {
        println!("  bpab: WARN (best={:.4}, max=8.0)", stats.min_bpb);
        GateStatus::Warn
    } else {
        println!("  bpab: FAIL (best={:.4}, max=8.0)", stats.min_bpb);
        GateStatus::Fail
    };
    results.insert("bpab", bpab_status);

    let top = lb.top(1)?;
    let size_status = if let Some(entry) = top.first() {
        if (entry.params as f64) <= 1_000_000.0 {
            println!("  size: PASS ({} params, max=1M)", entry.params);
            GateStatus::Pass
        } else {
            println!("  size: FAIL ({} params, max=1M)", entry.params);
            GateStatus::Fail
        }
    } else {
        println!("  size: no data (max=1M params)");
        GateStatus::Unknown
    };
    results.insert("size", size_status);

    let time_status = if let Some(entry) = top.first() {
        if entry.time_sec <= 3600.0 {
            println!("  time: PASS ({:.1}s, max=3600s)", entry.time_sec);
            GateStatus::Pass
        } else {
            println!("  time: FAIL ({:.1}s, max=3600s)", entry.time_sec);
            GateStatus::Fail
        }
    } else {
        println!("  time: no data (max=3600s)");
        GateStatus::Unknown
    };
    results.insert("time", time_status);

    let overall = if results.values().all(|s| *s == GateStatus::Pass) {
        GateStatus::Pass
    } else if results.values().any(|s| *s == GateStatus::Fail) {
        GateStatus::Fail
    } else if results.values().any(|s| *s == GateStatus::Warn) {
        GateStatus::Warn
    } else {
        GateStatus::Unknown
    };

    println!(
        "  {} entries in leaderboard, avg BPB={:.4}",
        stats.count, stats.avg_bpb
    );

    Ok(overall)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateStatus {
    Pass,
    Warn,
    Fail,
    Unknown,
}
