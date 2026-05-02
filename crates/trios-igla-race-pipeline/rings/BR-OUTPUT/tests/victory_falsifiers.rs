//! Verbatim port of the L7 IGLA Victory Gate falsifiers from
//! `trios-igla-race::victory::tests`. Required by acceptance criteria
//! of #459.
//!
//! Every test here is either a positive admission case or a Popper-
//! razor falsification witness (R8). If any falsifier ever passes,
//! INV-7 is empirically refuted.

use trios_igla_race_pipeline_br_output::{
    check_victory, is_victory, stat_strength, BPB_VICTORY_TARGET, INV2_WARMUP_BLIND_STEPS,
    JEPA_PROXY_BPB_FLOOR, SeedResult, TTEST_ALPHA, VICTORY_SEED_TARGET, VictoryError,
};

fn mk(seed: u64, bpb: f64) -> SeedResult {
    SeedResult {
        seed,
        bpb,
        step: INV2_WARMUP_BLIND_STEPS + 1,
        sha: "deadbeef".into(),
    }
}

#[test]
fn admit_three_distinct_seeds_below_target() {
    let r = vec![mk(1, 1.49), mk(2, 1.45), mk(3, 1.40)];
    let report = check_victory(&r).expect("expected victory");
    assert_eq!(report.winning_seeds, vec![1, 2, 3]);
    assert!((report.min_bpb - 1.40).abs() < 1e-12);
    assert!((report.mean_bpb - (1.49 + 1.45 + 1.40) / 3.0).abs() < 1e-12);
}

#[test]
fn admit_seed_ordering_invariant() {
    let asc = vec![mk(1, 1.49), mk(2, 1.45), mk(3, 1.40)];
    let desc = vec![mk(3, 1.40), mk(2, 1.45), mk(1, 1.49)];
    assert_eq!(check_victory(&asc), check_victory(&desc));
}

#[test]
fn falsify_two_seeds_insufficient() {
    let r = vec![mk(1, 1.49), mk(2, 1.45)];
    match check_victory(&r) {
        Err(VictoryError::InsufficientSeeds {
            passing_distinct,
            required,
        }) => {
            assert_eq!(passing_distinct, 2);
            assert_eq!(required, VICTORY_SEED_TARGET as usize);
        }
        other => panic!("expected InsufficientSeeds, got {other:?}"),
    }
}

#[test]
fn falsify_bpb_equal_target_strict_lt() {
    let r = vec![
        mk(1, BPB_VICTORY_TARGET),
        mk(2, BPB_VICTORY_TARGET),
        mk(3, BPB_VICTORY_TARGET),
    ];
    assert!(matches!(
        check_victory(&r),
        Err(VictoryError::BpbAboveTarget { .. }) | Err(VictoryError::InsufficientSeeds { .. })
    ));
}

#[test]
fn falsify_jepa_proxy_bpb() {
    let r = vec![mk(1, 0.014), mk(2, 1.45), mk(3, 1.40)];
    match check_victory(&r) {
        Err(VictoryError::JepaProxyDetected { seed, bpb }) => {
            assert_eq!(seed, 1);
            assert!(bpb < JEPA_PROXY_BPB_FLOOR);
        }
        other => panic!("expected JepaProxyDetected, got {other:?}"),
    }
}

#[test]
fn falsify_duplicate_seed_rejected() {
    let r = vec![mk(42, 1.49), mk(42, 1.45), mk(7, 1.40)];
    assert_eq!(
        check_victory(&r),
        Err(VictoryError::DuplicateSeed { seed: 42 })
    );
}

#[test]
fn falsify_pre_warmup_step_rejected() {
    let r = vec![
        SeedResult {
            seed: 1,
            bpb: 1.49,
            step: INV2_WARMUP_BLIND_STEPS - 1,
            sha: "d".into(),
        },
        mk(2, 1.45),
        mk(3, 1.40),
    ];
    match check_victory(&r) {
        Err(VictoryError::BeforeWarmup { step, warmup, .. }) => {
            assert_eq!(step, INV2_WARMUP_BLIND_STEPS - 1);
            assert_eq!(warmup, INV2_WARMUP_BLIND_STEPS);
        }
        other => panic!("expected BeforeWarmup, got {other:?}"),
    }
}

#[test]
fn falsify_non_finite_bpb_rejected() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let r = vec![mk(1, bad), mk(2, 1.45), mk(3, 1.40)];
        match check_victory(&r) {
            Err(VictoryError::NonFiniteBpb { seed: 1, .. }) => {}
            other => panic!("expected NonFiniteBpb for {bad}, got {other:?}"),
        }
    }
}

#[test]
fn falsify_padded_with_non_passing_still_insufficient() {
    let r = vec![mk(1, 1.49), mk(2, 1.45), mk(3, 1.51), mk(4, 1.60)];
    match check_victory(&r) {
        Err(VictoryError::BpbAboveTarget { target, .. }) => {
            assert!((target - BPB_VICTORY_TARGET).abs() < f64::EPSILON);
        }
        other => panic!("expected BpbAboveTarget, got {other:?}"),
    }
}

#[test]
fn falsify_at_jepa_floor_is_proxy() {
    let just_below = JEPA_PROXY_BPB_FLOOR - 1e-9;
    let r = vec![mk(1, just_below), mk(2, 1.45), mk(3, 1.40)];
    assert!(matches!(
        check_victory(&r),
        Err(VictoryError::JepaProxyDetected { .. })
    ));
    let r2 = vec![mk(1, JEPA_PROXY_BPB_FLOOR), mk(2, 1.45), mk(3, 1.40)];
    let report = check_victory(&r2).expect("floor value is admissible");
    assert!(report.winning_seeds.contains(&1));
}

#[test]
fn is_victory_agrees_with_check_victory() {
    let win = vec![mk(1, 1.49), mk(2, 1.45), mk(3, 1.40)];
    let lose = vec![mk(1, 1.49), mk(2, 1.45)];
    assert!(is_victory(&win));
    assert!(!is_victory(&lose));
}

#[test]
fn trinity_seed_target_is_three() {
    const _: () = assert!(VICTORY_SEED_TARGET == 3);
}

#[test]
fn igla_target_bpb_pinned_to_1_5() {
    assert!((BPB_VICTORY_TARGET - 1.5).abs() < f64::EPSILON);
}

#[test]
fn ttest_rejects_when_p_value_above_alpha() {
    let r = vec![
        SeedResult { seed: 42, bpb: 1.55, step: 5000, sha: "a".into() },
        SeedResult { seed: 43, bpb: 1.55, step: 5000, sha: "b".into() },
        SeedResult { seed: 44, bpb: 1.55, step: 5000, sha: "c".into() },
    ];
    match stat_strength(&r) {
        Err(VictoryError::TtestFailed { t_statistic, p_value, alpha }) => {
            assert!(p_value >= TTEST_ALPHA);
            assert!((alpha - TTEST_ALPHA).abs() < f64::EPSILON);
            assert!(t_statistic >= 0.0);
        }
        other => panic!("expected TtestFailed, got {other:?}"),
    }
}

#[test]
fn ttest_passes_when_distribution_clearly_below_baseline() {
    let r = vec![
        SeedResult { seed: 42, bpb: 1.40, step: 5000, sha: "a".into() },
        SeedResult { seed: 43, bpb: 1.39, step: 5000, sha: "b".into() },
        SeedResult { seed: 44, bpb: 1.41, step: 5000, sha: "c".into() },
    ];
    let report = stat_strength(&r).expect("expected t-test pass");
    assert!(report.passed);
    assert!(report.p_value < TTEST_ALPHA);
    assert!(report.t_statistic < 0.0);
}

#[test]
fn gate_final_check_victory_on_3_row_tail() {
    let r = vec![
        SeedResult { seed: 42, bpb: 1.42, step: 5000, sha: "a".into() },
        SeedResult { seed: 43, bpb: 1.44, step: 5000, sha: "b".into() },
        SeedResult { seed: 44, bpb: 1.40, step: 5000, sha: "c".into() },
    ];
    let report = check_victory(&r).expect("3 Gate-final seeds below 1.5");
    assert_eq!(report.winning_seeds, vec![42, 43, 44]);
    assert!(report.mean_bpb < 1.5);
}

#[test]
fn falsify_inv7_rejects_set() {
    let two = vec![
        SeedResult { seed: 42, bpb: 1.42, step: 5000, sha: "a".into() },
        SeedResult { seed: 43, bpb: 1.44, step: 5000, sha: "b".into() },
    ];
    assert!(check_victory(&two).is_err(), "2 seeds must be rejected");

    let dup = vec![
        SeedResult { seed: 42, bpb: 1.42, step: 5000, sha: "a".into() },
        SeedResult { seed: 42, bpb: 1.44, step: 5000, sha: "b".into() },
        SeedResult { seed: 43, bpb: 1.40, step: 5000, sha: "c".into() },
    ];
    assert!(check_victory(&dup).is_err(), "duplicate seed must be rejected");

    let one_above = vec![
        SeedResult { seed: 42, bpb: 1.42, step: 5000, sha: "a".into() },
        SeedResult { seed: 43, bpb: 1.44, step: 5000, sha: "b".into() },
        SeedResult { seed: 44, bpb: 1.55, step: 5000, sha: "c".into() },
    ];
    assert!(
        check_victory(&one_above).is_err(),
        "seed with bpb >= 1.5 must be rejected"
    );
}

#[test]
fn gate_final_stat_strength_on_3_seeds() {
    let r = vec![
        SeedResult { seed: 42, bpb: 1.35, step: 81000, sha: "a".into() },
        SeedResult { seed: 43, bpb: 1.38, step: 81000, sha: "b".into() },
        SeedResult { seed: 44, bpb: 1.32, step: 81000, sha: "c".into() },
    ];
    let report = stat_strength(&r).expect("Gate-final 3-seed stat strength");
    assert!(report.passed);
    assert!(report.p_value < 0.01);
}
