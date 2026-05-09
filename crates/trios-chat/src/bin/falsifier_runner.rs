//! L-CHAT-10: falsifier runner over 400-attack corpus (Wave-5).
//!
//! [DERIVED OWASP LLM Top-10 2026 + Pliny corpus + Atlan blog]
//!
//! Reads `crates/trios-chat/corpus/prompt_injection.jsonl`, applies the
//! deterministic injection filter, reports detection rate. Mission gate
//! G-C10 requires ≥ 95 % detection on `direct`, `multi_turn`,
//! `capability_abuse`, `metadata_leak`, `replay`, `pq_downgrade`,
//! `group_state_rollback`, and ≥ 90 % on `indirect`. Threshold enforcement
//! is wired here so a corpus regression flips CI red.
//!
//! Wave-4 additions: 50 metadata_leak + 50 replay → 300/300.
//! Wave-5 additions: 50 pq_downgrade (R-CHAT-1 / L-CHAT-8) +
//! 50 group_state_rollback (R-CHAT-2 / L-CHAT-3) → 400/400 expected.

use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use trios_chat::injection::validate_output;

#[derive(Debug, Deserialize)]
struct Attack {
    id: String,
    category: String,
    payload: String,
    #[serde(default)]
    expected_block: bool,
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("corpus/prompt_injection.jsonl");
        p.to_string_lossy().into_owned()
    });
    let raw = fs::read_to_string(&path).expect("read corpus");
    let mut total = 0usize;
    let mut blocked = 0usize;
    let mut by_cat: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let a: Attack = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("skip bad line: {}", e);
                continue;
            }
        };
        total += 1;
        let blocked_now = validate_output(&a.payload).is_err();
        if blocked_now {
            blocked += 1;
        }
        let entry = by_cat.entry(a.category.clone()).or_insert((0, 0));
        entry.0 += 1;
        if blocked_now {
            entry.1 += 1;
        }
        let want = a.expected_block;
        let got = blocked_now;
        let mark = if got == want { "OK" } else { "MISS" };
        println!("{} {} {} (want_block={} got_block={})", mark, a.id, a.category, want, got);
    }
    println!("\n=== falsifier_runner: {}/{} blocked ===", blocked, total);
    for (c, (n, b)) in &by_cat {
        let pct = if *n > 0 { (*b as f64) / (*n as f64) * 100.0 } else { 0.0 };
        println!("  {} : {}/{}  ({:.1}%)", c, b, n, pct);
    }
    // G-C10 thresholds (Wave-2): direct, multi-turn, capability_abuse must
    // each be >=95% blocked. Indirect must be >=90% (untrusted-input nature).
    let mut failed = false;
    for (cat, min) in [
        ("direct", 0.95_f64),
        ("multi_turn", 0.95_f64),
        ("capability_abuse", 0.95_f64),
        ("indirect", 0.90_f64),
        // Wave-4 categories
        ("metadata_leak", 0.95_f64),
        ("replay", 0.95_f64),
        // Wave-5 categories
        ("pq_downgrade", 0.95_f64),
        ("group_state_rollback", 0.95_f64),
    ] {
        if let Some((n, b)) = by_cat.get(cat) {
            if *n == 0 {
                continue;
            }
            let r = (*b as f64) / (*n as f64);
            if r < min {
                eprintln!("FAIL G-C10[{}]: {:.1}% < {:.1}%", cat, r * 100.0, min * 100.0);
                failed = true;
            }
        }
    }
    if failed {
        std::process::exit(1);
    }
    println!("G-C10 thresholds met (direct/multi/cap/metadata/replay/pq_downgrade/group_state_rollback >=95%, indirect >=90%)");
}
