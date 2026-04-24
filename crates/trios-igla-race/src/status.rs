//! Race status and leaderboard display

use tokio_postgres::Client;
use anyhow::Result;

/// Print race leaderboard from Neon
pub async fn print_leaderboard(client: &Client) -> Result<()> {
    let rows = client.query(
        "SELECT trial_id::text, machine_id, config::text, status, COALESCE(final_bpb::text, '-'), COALESCE(final_step::text, '-'), COALESCE(CASE WHEN rung_27000_bpb IS NOT NULL THEN '27000' WHEN rung_9000_bpb IS NOT NULL THEN '9000' WHEN rung_3000_bpb IS NOT NULL THEN '3000' WHEN rung_1000_bpb IS NOT NULL THEN '1000' ELSE '0' END, '0')::text as best_rung, COALESCE((SELECT lesson FROM igla_race_experience e WHERE e.trial_id::text = t.trial_id::text ORDER BY e.pattern_count DESC LIMIT 1), '-') as lesson, COALESCE((SELECT COUNT(*) + 1 FROM igla_race_trials t2 WHERE t2.final_bpb::float8 < t.final_bpb::float8), 999999)::bigint::text as bpb_rank FROM igla_race_trials t WHERE t.status IN ('completed', 'pruned') AND t.final_bpb IS NOT NULL ORDER BY t.final_bpb ASC NULLS LAST LIMIT 20",
        &[],
    ).await?;

    println!("IGLA RACE LEADERBOARD");
    println!("Rank |  BPB   | Steps |  Rung  | Machine      | Lesson");

    for row in rows.iter() {
        let _trial_id = row.try_get::<usize, &str>(0).unwrap_or(&"-");
        let machine_id = row.try_get::<usize, &str>(1).unwrap_or(&"-");
        let _config_json = row.try_get::<usize, &str>(2).unwrap_or(&"{}");
        let _status = row.try_get::<usize, &str>(3).unwrap_or(&"-");
        let final_bpb = row.try_get::<usize, &str>(4).unwrap_or(&"-");
        let final_step = row.try_get::<usize, &str>(5).unwrap_or(&"-");
        let best_rung = row.try_get::<usize, &str>(6).unwrap_or(&"0");
        let lesson = row.try_get::<usize, &str>(7).unwrap_or(&"-");
        let bpb_rank = row.try_get::<usize, &str>(8).unwrap_or(&"999999");

        let rank = if bpb_rank == "999999" { "-".to_string() } else { format!("#{}", bpb_rank) };

        let machine_trunc = if machine_id.len() > 10 {
            format!("{}...", &machine_id[..7])
        } else {
            machine_id.to_string()
        };

        let lesson_trunc = if lesson.len() > 28 {
            format!("{}...", &lesson[..25])
        } else {
            lesson.to_string()
        };

        println!("{} | {:6} | {:5} | {:6} | {:11} | {:30}",
                 rank, final_bpb, final_step, best_rung, machine_trunc, lesson_trunc);
    }

    println!();
    println!("Top Patterns (Failure Memory):");
    println!("Type    | Count | Pattern");

    let patterns = client.query(
        "SELECT lesson_type, lesson, pattern_count::text FROM igla_race_experience WHERE pattern_count > 0 ORDER BY pattern_count DESC, confidence DESC LIMIT 10",
        &[],
    ).await?;

    for row in patterns.iter() {
        let lesson_type = row.try_get::<usize, &str>(0).unwrap_or(&"-");
        let count = row.try_get::<usize, &str>(1).unwrap_or(&"0");
        let lesson = row.try_get::<usize, &str>(2).unwrap_or(&"-");

        let lesson_trunc = if lesson.len() > 58 {
            format!("{}...", &lesson[..55])
        } else {
            lesson.to_string()
        };

        println!("{} | {:5} | {}", lesson_type, count, lesson_trunc);
    }

    Ok(())
}

/// Print best trial
pub async fn print_best(client: &Client) -> Result<()> {
    let row = client.query_one(
        "SELECT trial_id::text, machine_id, config::text, final_bpb::text, final_step::text FROM igla_race_trials WHERE final_bpb IS NOT NULL ORDER BY final_bpb ASC LIMIT 1",
        &[],
    ).await;

    if let Ok(row) = row {
        let _trial_id = row.try_get::<usize, &str>(0).unwrap_or(&"-");
        let machine_id = row.try_get::<usize, &str>(1).unwrap_or(&"-");
        let config_json = row.try_get::<usize, &str>(2).unwrap_or(&"{}");
        let final_bpb = row.try_get::<usize, &str>(3).unwrap_or(&"-");
        let final_step = row.try_get::<usize, &str>(4).unwrap_or(&"-");

        let config: serde_json::Value = serde_json::from_str(config_json).unwrap_or(serde_json::json!({}));
        let config_str = serde_json::to_string_pretty(&config).unwrap_or_else(|_| "?".to_string());

        println!("BEST TRIAL");
        println!("Machine: {}", machine_id);
        println!("BPB: {}", final_bpb);
        println!("Steps: {}", final_step);
        println!("Config: {}", config_str);
    } else {
        println!("No trials completed yet");
    }

    Ok(())
}
