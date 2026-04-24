//! Race status and leaderboard display

use sqlx::PgPool;
use serde::Deserialize;
use anyhow::Result;

/// Leaderboard entry
#[derive(Debug, Deserialize)]
pub struct LeaderboardEntry {
    pub trial_id: String,
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
pub async fn show_status(pool: &PgPool) -> Result<()> {
    let entries = sqlx::query_as!(
        r#"
        SELECT 
            trial_id::text as trial_id,
            machine_id as machine_id,
            config as "config!",
            status as status,
            final_bpb as "final_bpb?",
            final_step as "final_step?",
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
             ORDER BY e.pattern_count DESC LIMIT 1) as "lesson?",
            COALESCE(
                (SELECT COUNT(*) + 1 FROM igla_race_trials t2 
                 WHERE t2.final_bpb < t.final_bpb),
                999999
            )::bigint as bpb_rank
        FROM igla_race_trials t
        WHERE t.status IN ('completed', 'pruned') AND t.final_bpb IS NOT NULL
        ORDER BY t.final_bpb ASC NULLS LAST
        LIMIT 20
        "#
    )
    .fetch_all(pool)
    .await?;
    
    println!("┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│                    IGLA RACE LEADERBOARD                                 │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!("│ Rank │  BPB   │ Steps │  Rung  │ Machine      │ Lesson                         │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    
    for entry in entries {
        let rank = format!("#{}", entry.bpb_rank);
        let bpb = entry.final_bpb.map_or("-".to_string(), |b| format!("{:.4}", b));
        let steps = entry.final_step.map_or("-".to_string(), |s| format!("{}", s));
        let rung = entry.best_rung;
        let machine = &entry.machine_id;
        let lesson = entry.lesson.as_deref().unwrap_or(&"-".to_string());
        
        // Truncate if too long
        let machine_trunc = if machine.len() > 10 {
            &machine[..10]
        } else {
            machine
        };
        let lesson_trunc = if lesson.len() > 28 {
            format!("{}...", &lesson[..25])
        } else {
            lesson.clone()
        };
        
        println!("│ {:4} │ {:6} │ {:5} │ {:6} │ {:11} │ {:30} │",
                 rank, bpb, steps, rung, machine_trunc, lesson_trunc);
    }
    
    println!("└─────────────────────────────────────────────────────────────────────────────┘");
    
    // Show top patterns/lessons
    println!("\nTop Patterns (Failure Memory):");
    println!("┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ Type    │ Count │ Pattern                                                      │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    
    let patterns = sqlx::query_as!(
        r#"
        SELECT lesson_type, lesson, pattern_count
        FROM igla_race_experience
        WHERE pattern_count > 0
        ORDER BY pattern_count DESC, confidence DESC
        LIMIT 10
        "#
    )
    .fetch_all(pool)
    .await?;
    
    for pattern in patterns {
        let lesson_type = &pattern.lesson_type;
        let count = pattern.pattern_count;
        let lesson = &pattern.lesson;
        let lesson_trunc = if lesson.len() > 58 {
            format!("{}...", &lesson[..55])
        } else {
            lesson.clone()
        };
        
        println!("│ {:7} │ {:5} │ {:59} │", lesson_type, count, lesson_trunc);
    }
    
    println!("└─────────────────────────────────────────────────────────────────────────────┘");
    
    Ok(())
}

/// Show summary statistics
pub async fn show_summary(pool: &PgPool) -> Result<()> {
    let total_trials: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM igla_race_trials")
        .fetch_one(pool)
        .await?;
    
    let completed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM igla_race_trials WHERE status = 'completed'")
        .fetch_one(pool)
        .await?;
    
    let pruned: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM igla_race_trials WHERE status = 'pruned'")
        .fetch_one(pool)
        .await?;
    
    let running: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM igla_race_trials WHERE status = 'running'")
        .fetch_one(pool)
        .await?;
    
    let best_bpb: Option<f64> = sqlx::query_scalar(
        "SELECT MIN(final_bpb) FROM igla_race_trials WHERE final_bpb IS NOT NULL"
    )
    .fetch_one(pool)
    .await?;
    
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
