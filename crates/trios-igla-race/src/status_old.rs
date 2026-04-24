//! Race status and leaderboard display

use tokio_postgres::Client;
use anyhow::Result;

/// Print race leaderboard from Neon
pub async fn print_leaderboard(client: &Client) -> Result<()> {
    let rows = client.query(
        "SELECT trial_id::text, machine_id, COALESCE(final_bpb, '-'), COALESCE(final_step, '-'), lesson, bpb_rank
         FROM igla_race_leaderboard LIMIT 20",
        &[],
    ).await?;

    println!("IGLA RACE LEADERBOARD");
    println!("Rank | BPB   | Steps | Machine");
    println!("-----|--------|-------|--------");

    for row in rows.iter() {
        let _trial_id: String = row.get(0)?;
        let machine_id: String = row.get(1)?;
        let final_bpb: String = row.get(2)?;
        let final_step: String = row.get(3)?;
        let lesson: String = row.get(4)?;
        let bpb_rank: i64 = row.get(5)?;

        let rank = if bpb_rank == 999999 { "-".to_string() } else { format!("#{}", bpb_rank) };

        println!("{} | {} | {} | {} | {}", rank, final_bpb, final_step, machine_id, lesson);
    }

    Ok(())
}

/// Print best trial
pub async fn print_best(client: &Client) -> Result<()> {
    let row = client.query_one(
        "SELECT trial_id::text, machine_id, config::text, final_bpb::text, final_step::text FROM igla_race_trials WHERE final_bpb IS NOT NULL ORDER BY final_bpb ASC LIMIT 1",
        &[],
    ).await?;

    if let Some(row) = row {
        let _trial_id: String = row.get(0)?;
        let machine_id: String = row.get(1)?;
        let config_json: String = row.get(2)?;
        let final_bpb: String = row.get(3)?;
        let final_step: String = row.get(4)?;

        let config: serde_json::Value = serde_json::from_str(&config_json).unwrap_or(serde_json::json!({}));
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
