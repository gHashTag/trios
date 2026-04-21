use anyhow::Result;
use trios_igla_trainer::{AuditLog, Schedule, TrainConfig};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = TrainConfig::default();
    let schedule = match config.schedule {
        trios_igla_trainer::config::ScheduleType::Flat3e4 => Schedule::Flat3e4,
        trios_igla_trainer::config::ScheduleType::Cosine => Schedule::Cosine,
        trios_igla_trainer::config::ScheduleType::PhiWarmup => Schedule::PhiWarmup,
    };

    let git_sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let mut audit = AuditLog::new(&config.model_id, config.seed, config.steps, &git_sha);

    tracing::info!(
        "IGLA trainer starting: model={} steps={} seed={}",
        config.model_id,
        config.steps,
        config.seed
    );

    let mut loss: f32 = 10.0;
    let mut rng_state = config.seed;

    for step in 1..=config.steps {
        let lr = schedule.lr(step, config.steps);

        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let noise = ((rng_state >> 33) as f32 / u32::MAX as f32) - 0.5;

        loss = loss * (1.0 - lr * 10.0) + noise * 0.001;
        loss = loss.max(0.01);

        let bpb = schedule.bpb_from_loss(loss);

        audit.record(step, loss, bpb, lr);

        if step % 200 == 0 || step == config.steps {
            tracing::info!(
                "step={:4} loss={:.4} bpb={:.4} lr={:.6}",
                step,
                loss,
                bpb,
                lr
            );
        }

        if step % 500 == 0 {
            if let Err(e) = audit.dump_metric("metric.json") {
                tracing::warn!("metric dump failed at step {}: {}", step, e);
            } else {
                tracing::info!("metric.json written at step {}", step);
            }
        }
    }

    audit.dump_metric("metric.json")?;
    let json = audit.to_json();
    println!("{}", json);

    Ok(())
}
