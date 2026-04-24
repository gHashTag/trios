//! Race status and leaderboard display

use crate::neon::NeonDb;
use anyhow::Result;
use uuid::Uuid;

/// Leaderboard entry
#[derive(Debug, Clone)]
pub struct LeaderboardEntry {
    pub trial_id: Uuid,
    pub machine_id: String,
    pub config: serde_json::Value,
    pub status: String,
    pub final_bpb: Option<f64>,
    pub final_step: Option<i32>,
    pub best_rung: i32,
    pub lesson: Option<String>,
    pub bpb_rank: i64,
}

/// Display race status
pub async fn show_status(db: &NeonDb) -> Result<()> {
    let rows = db.client().query(
        r#"
        SELECT
            trial_id,
            machine_id,
            config,
            status,
            final_bpb,
            final_step,
            COALESCE(
                CASE
                    WHEN rung_27000_bpb IS NOT NULL THEN 27000
                    WHEN rung_9000_bpb IS NOT NULL THEN 9000
                    WHEN rung_3000_bpb IS NOT NULL THEN 3000
                    WHEN rung_1000_bpb IS NOT NULL THEN 1000
                    ELSE 0
                END,
                0
            )::int as best_rung,
            (SELECT lesson FROM igla_race_experience e
             WHERE e.trial_id = t.trial_id
             ORDER BY e.pattern_count DESC LIMIT 1) as lesson,
            COALESCE(
                (SELECT COUNT(*) + 1 FROM igla_race_trials t2
                 WHERE t2.final_bpb < t.final_bpb),
                999999
            )::bigint as bpb_rank
        FROM igla_race_trials t
        WHERE t.status IN ('completed', 'pruned') AND t.final_bpb IS NOT NULL
        ORDER BY t.final_bpb ASC NULLS LAST
        LIMIT 20
        "#,
        &[]
    ).await?;

    println!("┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│                    IGLA RACE LEADERBOARD                                 │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!("│ Rank │  BPB   │ Steps │  Rung  │ Machine      │ Lesson                         │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");

    for row in rows {
        let trial_id: Uuid = row.get(0);
        let machine_id: String = row.get(1);
        let config_json: String = row.get(2);
        let config: serde_json::Value = serde_json::from_str(&config_json).unwrap_or(serde_json::json!({}));
        let status: String = row.get(3);
        let final_bpb: Option<f64> = row.get(4);
        let final_step: Option<i32> = row.get(5);
        let best_rung: i32 = row.get(6);
        let lesson: Option<String> = row.get(7);
        let bpb_rank: i64 = row.get(8);

        let rank = format!("#{}", bpb_rank);
        let bpb = final_bpb.map_or("-".to_string(), |b| format!("{:.4}", b));
        let steps = final_step.map_or("-".to_string(), |s| format!("{}", s));

        // Truncate machine if too long
        let machine_trunc = if machine_id.len() > 10 {
            format!("{}...", &machine_id[..7])
        } else {
            machine_id.clone()
        };

        // Truncate lesson if too long
        let lesson_display = lesson.as_deref().unwrap_or("-");
        let lesson_trunc = if lesson_display.len() > 28 {
            format!("{}...", &lesson_display[..25])
        } else {
            lesson_display.to_string()
        };

        println!("│ {:4} │ {:6} │ {:5} │ {:6} │ {:11} │ {:30} │",
                 rank, bpb, steps, best_rung, machine_trunc, lesson_trunc);
    }

    println!("└─────────────────────────────────────────────────────────────────────────────┘");

    // Show top patterns/lessons
    println!("\nTop Patterns (Failure Memory):");
    println!("┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ Type    │ Count │ Pattern                                                      │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");

    let patterns = db.client().query(
        r#"
        SELECT lesson_type, lesson, pattern_count
        FROM igla_race_experience
        WHERE pattern_count > 0
        ORDER BY pattern_count DESC, confidence DESC
        LIMIT 10
        "#,
        &[]
    ).await?;

    for row in patterns {
        let lesson_type: String = row.get(0);
        let count: i32 = row.get(1);
        let lesson: String = row.get(2);

        let lesson_trunc = if lesson.len() > 58 {
            format!("{}...", &lesson[..55])
        } else {
            lesson
        };

        println!("│ {:7} │ {:5} │ {:59} │", lesson_type, count, lesson_trunc);
    }

    println!("└─────────────────────────────────────────────────────────────────────────────┘");

    Ok(())
}

/// Show summary statistics
pub async fn show_summary(db: &NeonDb) -> Result<()> {
    let total_trials: i64 = db.client().query_one(
        "SELECT COUNT(*) FROM igla_race_trials",
        &[]
    ).await?.get(0);

    let completed: i64 = db.client().query_one(
        "SELECT COUNT(*) FROM igla_race_trials WHERE status = 'completed'",
        &[]
    ).await?.get(0);

    let pruned: i64 = db.client().query_one(
        "SELECT COUNT(*) FROM igla_race_trials WHERE status = 'pruned'",
        &[]
    ).await?.get(0);

    let running: i64 = db.client().query_one(
        "SELECT COUNT(*) FROM igla_race_trials WHERE status = 'running'",
        &[]
    ).await?.get(0);

    let best_bpb_row = db.client().query_one(
        "SELECT MIN(final_bpb) FROM igla_race_trials WHERE final_bpb IS NOT NULL",
        &[]
    ).await?;
    let best_bpb: Option<f64> = best_bpb_row.get(0);

    println!("┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│                    IGLA RACE SUMMARY                                       │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!("│ Total Trials:    {:>6}                                                          │", total_trials);
    println!("│ Completed:       {:>6}                                                          │", completed);
    println!("│ Pruned:          {:>6}                                                          │", pruned);
    println!("│ Running:         {:>6}                                                          │", running);
    println!("│ Best BPB:        {:>6}                                                          │",
             best_bpb.map_or("-".to_string(), |b| format!("{:.4}", b)));
    println!("└─────────────────────────────────────────────────────────────────────────────┘");

    Ok(())
}
