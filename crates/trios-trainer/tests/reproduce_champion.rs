//! Champion reproduction test
//!
//! Validates that trios-trainer can reproduce the champion baseline:
//! commit 2446855 → BPB = 2.2393 ± 0.01 @ 27K steps, seed=43

#[test]
fn test_champion_config_loads() {
    let config = trios_trainer::Config::load("configs/champion.toml")
        .expect("champion.toml should load");

    assert_eq!(config.training.seed, 43);
    assert_eq!(config.training.steps, 27000);
    assert_eq!(config.training.lr, 0.004);
    assert_eq!(config.model.d_model, 384);

    // INV-8 validation
    assert!(trios_trainer::config::validate_lr_phi_band(config.training.lr));
}

#[test]
fn test_inv8_lr_validation() {
    // Valid LR values
    assert!(trios_trainer::config::validate_lr_phi_band(0.001));
    assert!(trios_trainer::config::validate_lr_phi_band(0.004));
    assert!(trios_trainer::config::validate_lr_phi_band(0.01));

    // Invalid LR values
    assert!(!trios_trainer::config::validate_lr_phi_band(0.0009));
    assert!(!trios_trainer::config::validate_lr_phi_band(0.011));
}

#[test]
fn test_gate2_config_loads() {
    let config = trios_trainer::Config::load("configs/gate2-attempt.toml")
        .expect("gate2-attempt.toml should load");

    assert_eq!(config.training.seed, 43);
    assert_eq!(config.training.steps, 4000);

    // JEPA config present
    assert!(config.jepa.is_some());
    let jepa = config.jepa.as_ref().unwrap();
    assert_eq!(jepa.mask_ratio, 0.30);
}

#[test]
fn test_embargo_block() {
    let embargo = trios_trainer::ledger::EmbargoBlock {
        blocked_shas: vec!["deadbeef".into()],
    };

    assert!(embargo.is_embargoed("deadbeef"));
    assert!(!embargo.is_embargoed("goodcommit"));
}

#[test]
fn test_ledger_row_serialization() {
    let row = trios_trainer::ledger::LedgerRow {
        agent: "test".into(),
        bpb: 2.2393,
        seed: 43,
        sha: "abc123".into(),
        step: 27000,
        ts: "2026-04-26T00:00:00Z".into(),
        gate_status: "pending".into(),
    };

    let jsonl = serde_json::to_string(&row).unwrap();
    assert!(jsonl.contains("\"bpb\":2.2393"));
    assert!(jsonl.contains("\"seed\":43"));
    assert!(jsonl.contains("\"step\":27000"));
    assert!(jsonl.contains("\"agent\":\"test\""));
}

// Full champion reproduction test (ignored by default, run manually with --ignored)
//
// To run after PR-2 migration:
// ```bash
// cargo test -p trios-trainer reproduce_champion_full -- --ignored
// ```
#[test]
#[ignore]
fn reproduce_champion_full() {
    // TODO: After PR-2, this will run a full 27K-step training
    // and assert final_bpb ∈ [2.229, 2.249]

    let config = trios_trainer::Config::load("configs/champion.toml")
        .expect("champion.toml should load");

    let result = trios_trainer::run(&config)
        .expect("training should complete");

    // Champion tolerance: ±0.01 BPB
    assert!((2.229..=2.249).contains(&result.final_bpb),
        "BPB {} is outside champion tolerance [2.229, 2.249]", result.final_bpb);
}
