        for rung in AshaRung::all() {
            let step = rung.step();
            let bpb = simulate_training(&config, step as u64).await?;

            db.client().execute(
                &format!("UPDATE igla_race_trials SET rung_{}_step = $1, rung_{}_bpb = $2, final_step = $1, final_bpb = $2 WHERE trial_id = $3", step, step),
                &[&(step as i32), &(bpb as i32), &trial_id]
            ).await?;

            if bpb > 2.7 && step == 1000 {
                db.client().execute(
                    "UPDATE igla_race_trials SET status = 'pruned' WHERE trial_id = $1",
                    &[&trial_id]
                ).await?;
                pruned = true;
                break;
            }

            if bpb < 1.5 {
                db.client().execute(
                    "UPDATE igla_race_trials SET status = 'completed', final_bpb = $1 WHERE trial_id = $2",
                    &[&bpb, &trial_id]
                ).await?;
                return Ok(bpb);
            }

            prev_bpb = bpb;
        }

        if !pruned {
            db.client().execute(
                "UPDATE igla_race_trials SET status = 'completed', final_bpb = $1 WHERE trial_id = $2",
                &[&prev_bpb, &trial_id]
            ).await?;
        }
