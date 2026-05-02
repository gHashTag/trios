//! Integration test: 1 fake seed, full pipeline loop, completes in
//! < 60 s, BPB row written, gardener decision recorded.
//!
//! Acceptance criterion from #459. Trainer is `MockedTrainer::winning`
//! — descending BPB curve that crosses `BPB_VICTORY_TARGET` shortly
//! before the warmup window ends, so the gardener fires `Noop` /
//! eventual scarab `Done`.

use std::time::Instant;

use trios_igla_race_pipeline_br_output::{
    test_sinks::{VecBpbSink, VecGardenerSink},
    IglaRacePipeline, MockedTrainer, PipelineCfg, PipelineErr, Sinks,
    BPB_VICTORY_TARGET, INV2_WARMUP_BLIND_STEPS,
};
use trios_igla_race_pipeline_sr_00::{Seed, StrategyId, WorkerId};

#[tokio::test(flavor = "current_thread")]
async fn integration_one_fake_seed_completes_under_60s() {
    let start = Instant::now();

    let cfg = PipelineCfg {
        strategy_id: StrategyId::new(),
        worker_id: WorkerId::new("test-int", 0),
        seeds: vec![Seed(1597)],
        // small chunk → many iterations → exercises writer/gardener wiring
        steps_per_chunk: 250,
        // total steps that go just past warmup so gardener gets out of
        // INV-2 protection.
        total_steps: INV2_WARMUP_BLIND_STEPS as i64 + 1_000,
        trainer_config: serde_json::json!({"mock": true, "test": "integration"}),
    };

    let mut pipeline = IglaRacePipeline::new(cfg.clone());
    let mut bpb = VecBpbSink::default();
    let mut gd = VecGardenerSink::default();
    let mut tr = MockedTrainer::winning();

    let result = pipeline
        .run_e2e_ttt_o1(
            Sinks {
                bpb: &mut bpb,
                gardener: &mut gd,
            },
            &mut tr,
        )
        .await;

    let elapsed = start.elapsed();

    // R5-honest: a single seed cannot satisfy `VICTORY_SEED_TARGET = 3`.
    // The pipeline correctly returns `HonestNotYet`. The acceptance
    // criteria asks that BPB rows are written and gardener decisions
    // are recorded — both happen before the gate adjudicates.
    assert!(
        matches!(result, Err(PipelineErr::HonestNotYet { passing, required }) if required == 3 && passing <= 1),
        "expected HonestNotYet (1 seed, 3 required), got {result:?}"
    );

    // Acceptance: BPB rows were written.
    assert!(
        !bpb.rows.is_empty(),
        "no BPB rows captured by sink"
    );
    let final_row = bpb.rows.last().expect("at least one row");
    // The mock curve must converge below the target post-warmup.
    assert!(
        final_row.bpb < BPB_VICTORY_TARGET,
        "final BPB {} did not cross target {} — mock curve broken",
        final_row.bpb,
        BPB_VICTORY_TARGET
    );
    // Every row carries an EMA stamp from the writer.
    for row in &bpb.rows {
        assert!(row.ema.is_some(), "writer did not stamp EMA on row");
    }

    // Acceptance: gardener decisions recorded.
    assert!(
        !gd.decisions.is_empty(),
        "no gardener decisions captured by sink"
    );
    // Decisions are 1:1 with BPB rows.
    assert_eq!(
        gd.decisions.len(),
        bpb.rows.len(),
        "gardener/bpb sink count mismatch"
    );

    // Acceptance: < 60 s.
    assert!(
        elapsed.as_secs() < 60,
        "pipeline took {:?} (must be < 60s)",
        elapsed
    );
}

#[tokio::test(flavor = "current_thread")]
async fn integration_three_seeds_reach_victory() {
    // Same trainer, three Fibonacci seeds → 3 distinct passing seeds →
    // gate admits, returns VictoryReport.
    let cfg = PipelineCfg {
        strategy_id: StrategyId::new(),
        worker_id: WorkerId::new("test-int", 0),
        seeds: vec![Seed(1597), Seed(2584), Seed(4181)],
        steps_per_chunk: 500,
        total_steps: INV2_WARMUP_BLIND_STEPS as i64 + 500,
        trainer_config: serde_json::json!({"mock": true}),
    };
    let mut pipeline = IglaRacePipeline::new(cfg);
    let mut bpb = VecBpbSink::default();
    let mut gd = VecGardenerSink::default();
    let mut tr = MockedTrainer::winning();
    let report = pipeline
        .run_e2e_ttt_o1(
            Sinks {
                bpb: &mut bpb,
                gardener: &mut gd,
            },
            &mut tr,
        )
        .await
        .expect("3 winning seeds should pass INV-7");
    assert_eq!(report.winning_seeds.len(), 3);
    assert!(report.mean_bpb < BPB_VICTORY_TARGET);
}

#[tokio::test(flavor = "current_thread")]
async fn integration_losing_curve_returns_honest_not_yet() {
    // Same plumbing, losing trainer → gate returns InsufficientSeeds /
    // BpbAboveTarget which the pipeline maps to HonestNotYet.
    let cfg = PipelineCfg {
        strategy_id: StrategyId::new(),
        worker_id: WorkerId::new("test-int", 0),
        seeds: vec![Seed(1597), Seed(2584), Seed(4181)],
        steps_per_chunk: 500,
        total_steps: INV2_WARMUP_BLIND_STEPS as i64 + 500,
        trainer_config: serde_json::json!({"mock": true}),
    };
    let mut pipeline = IglaRacePipeline::new(cfg);
    let mut bpb = VecBpbSink::default();
    let mut gd = VecGardenerSink::default();
    let mut tr = MockedTrainer::losing();
    let result = pipeline
        .run_e2e_ttt_o1(
            Sinks {
                bpb: &mut bpb,
                gardener: &mut gd,
            },
            &mut tr,
        )
        .await;
    assert!(
        matches!(result, Err(PipelineErr::HonestNotYet { required: 3, .. })),
        "expected HonestNotYet, got {result:?}"
    );
    assert!(!bpb.rows.is_empty());
    assert!(!gd.decisions.is_empty());
}
