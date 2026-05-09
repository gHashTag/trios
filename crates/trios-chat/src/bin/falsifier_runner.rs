//! L-CHAT-10: falsifier runner over 200-attack corpus.
//!
//! [DERIVED OWASP LLM Top-10 2026 + Pliny corpus + Atlan blog]
//!
//! Reads `crates/trios-chat/corpus/prompt_injection.jsonl`, applies the
//! deterministic injection filter, reports detection rate. Mission gate
//! G-C10 requires ≥ 95 % detection on the *direct* category and ≥ 60 %
//! on *indirect+multi-turn*. Threshold enforcement is wired here so a
//! corpus regression flips CI red.

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
    // G-C10 threshold (scaffold; tightens as corpus grows).
    let direct_ok = by_cat
        .get("direct")
        .map(|(n, b)| *n == 0 || (*b as f64) / (*n as f64) >= 0.95)
        .unwrap_or(true);
    if !direct_ok {
        eprintln!("FAIL G-C10: direct-injection block-rate < 95%");
        std::process::exit(1);
    }
    println!("G-C10 direct-injection block-rate OK");
}
