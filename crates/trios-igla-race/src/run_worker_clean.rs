/// Single worker: runs trials until IGLA found
async fn run_worker(
    neon_url: &str,
    machine_id: &str,
    worker_id: u64,
    best_bpb: Arc<Mutex<f64>>,
) -> Result<f64> {
    let db = NeonDb::connect(neon_url).await?;
    let mut rng = rand::thread_rng();

    loop {
        let config = TrialConfig {
            seed: Some(rng.gen_range(40..1040)),
            d_model: Some(*[128, 192, 256, 384].choose(&mut rng).ok_or_else(|| anyhow::anyhow!("No d_model"))?),
            context: Some(*[4, 5, 6, 7, 8].choose(&mut rng).ok_or_else(|| anyhow::anyhow!("No context"))?),
            lr: Some(rng.gen_range(0.0001..0.01)),
            optimizer: Some(if rng.gen_bool(0.5) { "adamw" } else { "muon" }.to_string()),
            wd: Some(rng.gen_range(0.001..0.1)),
            use_attention: Some(rng.gen_bool(0.5)),
            hidden: Some(384),
            n_layers: Some(1),
            activation: Some("relu".to_string()),
            weight_decay: Some(0.01),
            dropout: Some(0.0),
            warmup_steps: Some(0),
            max_steps: Some(27000),
        };

        let config_value = serde_json::to_value(&config)?;

        let trial_id = db.register_trial(
            Uuid::new_v4(),
            machine_id,
            worker_id as i32,
            &config_value,
        ).await?;

        let mut prev_bpb = f64::MAX;
        let mut pruned = false;

        for rung in AshaRung::all() {
            let step = rung.step();
            let bpb = simulate_training(&config, step).await?;

            db.client().execute(
                &format!("UPDATE igla_race_trials SET rung_{}_step = $1, rung_{}_bpb = $2, final_step = $1, final_bpb = $2 WHERE trial_id = $3", step, step),
                &[&(step as i32), &bpb, &trial_id],
            ).await?;

            if bpb > 2.7 && step == 1000 {
                db.client().execute(
                    "UPDATE igla_race_trials SET status = 'pruned' WHERE trial_id = $1",
                    &[&trial_id],
                ).await?;
                pruned = true;
                break;
            }

            if bpb < 1.5 {
                db.client().execute(
                    "UPDATE igla_race_trials SET status = 'completed', final_bpb = $1 WHERE trial_id = $2",
                    &[&bpb, &trial_id],
                ).await?;
                return Ok(bpb);
            }

            prev_bpb = bpb;
        }

        if !pruned {
            db.client().execute(
                "UPDATE igla_race_trials SET status = 'completed', final_bpb = $1 WHERE trial_id = $2",
                &[&prev_bpb, &trial_id],
            ).await?;
        }

        {
            let mut b = best_bpb.lock().unwrap();
            if prev_bpb < *b {
                *b = prev_bpb;
            }
        }
    }
}
