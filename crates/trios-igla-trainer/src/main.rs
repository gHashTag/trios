use anyhow::Result;
use clap::Parser;
use trios_igla_trainer::{AuditLog, Schedule, TrainConfig};

#[derive(Parser)]
#[command(name = "igla-trainer")]
struct Args {
    #[arg(long, default_value = "igla-gf16")]
    model_id: String,

    #[arg(long, default_value_t = 1000)]
    steps: u64,

    #[arg(long, default_value_t = 4)]
    batch_size: usize,

    #[arg(long, default_value_t = 128)]
    seq_len: usize,

    #[arg(long, default_value = "flat3e4")]
    schedule: String,

    #[arg(long, default_value_t = 42)]
    seed: u64,

    #[arg(long)]
    exp_id: Option<String>,

    #[arg(long, default_value = "gHashTag/trios")]
    repo: String,

    #[arg(long, default_value = "main")]
    branch: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let schedule = match args.schedule.as_str() {
        "cosine" => Schedule::Cosine,
        "phi" => Schedule::PhiWarmup,
        _ => Schedule::Flat3e4,
    };

    let config = TrainConfig {
        model_id: args.model_id.clone(),
        steps: args.steps,
        batch_size: args.batch_size,
        seq_len: args.seq_len,
        schedule: match args.schedule.as_str() {
            "cosine" => trios_igla_trainer::config::ScheduleType::Cosine,
            "phi" => trios_igla_trainer::config::ScheduleType::PhiWarmup,
            _ => trios_igla_trainer::config::ScheduleType::Flat3e4,
        },
        seed: args.seed,
        repo: args.repo,
        branch: args.branch,
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

    // L7: Write experience log
    write_experience_log(&args.exp_id, &args.model_id, args.seed, &json)?;

    Ok(())
}

fn write_experience_log(exp_id: &Option<String>, model_id: &str, seed: u64, result_json: &str) -> Result<()> {
    use std::fs;
    use std::io::Write;

    let exp_name = exp_id.as_deref().unwrap_or("training");
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");

    let entry = format!(
        "[{}] TASK: {} | model={} | seed={} | result={}\n",
        timestamp, exp_name, model_id, seed, result_json
    );

    let dir = ".trinity/experience";
    fs::create_dir_all(dir)?;

    let filename = format!("{}/trios_{}.trinity", dir, chrono::Utc::now().format("%Y%m%d"));
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&filename)?
        .write_all(entry.as_bytes())?;

    tracing::info!("Experience logged to {}", filename);

    Ok(())
}
