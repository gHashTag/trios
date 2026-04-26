//! Champion reproduction test — P0 Audit Phase
//!
//! Validates that trios-trainer can reproduce champion baseline:
//! commit 2446855 → BPB = 2.2393 ± 0.01 @ 27K steps, seed=43

use trios_trainer::{Config, validate_lr_phi_band};

#[test]
fn test_champion_config_loads() {
    let config = Config::load("configs/champion.toml")
        .expect("champion.toml should load");

    assert_eq!(config.training.seed, 43);
    assert_eq!(config.training.steps, 27000);
    assert_eq!(config.training.lr, 0.004);
    assert_eq!(config.model.d_model, 384);

    // INV-8 validation
    assert!(validate_lr_phi_band(config.training.lr),
        "LR should be within φ-band [0.001, 0.01]");
}

#[test]
fn test_inv8_lr_validation() {
    // Valid LR values
    assert!(validate_lr_phi_band(0.001));
    assert!(validate_lr_phi_band(0.004));
    assert!(validate_lr_phi_band(0.01));

    // Invalid LR values
    assert!(!validate_lr_phi_band(0.0009));
    assert!(!validate_lr_phi_band(0.011));
}

#[test]
fn test_embargo_block() {
    let embargo = trios_trainer::ledger::EmbargoBlock::new();

    assert!(embargo.is_embargoed("deadbeef"));
    assert!(!embargo.is_embargoed("goodcommit"));
}

#[test]
fn test_ledger_row_serialization() {
    use trios_trainer::ledger::LedgerRow;
    use std::time::SystemTime;

    let row = LedgerRow {
        agent: "test".into(),
        bpb: 2.2393,
        seed: 43,
        sha: "abc123".into(),
        step: 27000,
        ts: "2026-04-27T00:00:00Z".into(),
        gate_status: "pending".into(),
    };

    let jsonl = serde_json::to_string(&row).unwrap();
    assert!(jsonl.contains("\"bpb\":2.2393"));
    assert!(jsonl.contains("\"seed\":43"));
    assert!(jsonl.contains("\"step\":27000"));
}

// Full champion reproduction test (ignored by default, requires full 27K-step run)
// To run after training infrastructure is complete:
// cargo test -p trios-trainer -- --ignored champion_reproduction
#[test]
#[ignore]
fn reproduce_champion_full() {
    // TODO: After full 27K-step training, this will:
    // 1. Run full training with champion.toml
    // 2. Validate final BPB ∈ [2.2293, 2.2493] (±0.01)
    // 3. Assert success
}
