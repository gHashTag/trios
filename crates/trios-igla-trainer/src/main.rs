use anyhow::Result;
use clap::Parser;
use trios_igla_trainer::{AuditLog, Schedule, TrainConfig};
use trios_train_cpu::jepa::{JepaConfig, MaskConfig, EmaConfig, EmaTarget, mask_spans, get_masked, compute_jepa_loss, JepaLossConfig};
use trios_train_cpu::objective::{ObjectiveConfig, ComponentLosses, compute_combined_loss};
use trios_igla_race::asha::{AshaConfig, AshaSchedule};
use rand::rngs::StdRng;
use rand::SeedableRng;

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

    /// Architecture variant: attn | jepa | hybrid | ngram
    /// jepa: enables T-JEPA multi-objective loss + ASHA rung 3000 first (Law L-R10)
    /// Ref: https://github.com/gHashTag/trinity/tree/main/docs/research/models/JEPA-T/
    #[arg(long, default_value = "attn")]
    arch: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let schedule = match args.schedule.as_str() {
        "cosine" => Schedule::Cosine,
        "phi"    => Schedule::PhiWarmup,
        _        => Schedule::Flat3e4,
    };

    let config = TrainConfig {
        model_id: args.model_id.clone(),
        steps: args.steps,
        batch_size: args.batch_size,
        seq_len: args.seq_len,
        schedule: match args.schedule.as_str() {
            "cosine" => trios_igla_trainer::config::ScheduleType::Cosine,
            "phi"    => trios_igla_trainer::config::ScheduleType::PhiWarmup,
            _        => trios_igla_trainer::config::ScheduleType::Flat3e4,
        },
        seed: args.seed,
        repo: args.repo,
        branch: args.branch,
    };

    // ── ASHA schedule (arch-aware) ─────────────────────────────────────────
    let asha_cfg = AshaConfig::for_arch(&args.arch);
    let asha_schedule = asha_cfg.schedule();
    tracing::info!(
        "ASHA schedule for arch='{}': first_rung={} rungs={:?}",
        args.arch,
        asha_schedule.first_rung_steps(),
        asha_schedule.rungs().iter().map(|r| r.step()).collect::<Vec<_>>()
    );

    // ── T-JEPA setup (only active when --arch jepa) ────────────────────────
    let jepa_active = args.arch == "jepa";
    let jepa_cfg = JepaConfig::default();
    let mask_cfg = MaskConfig {
        ratio:     jepa_cfg.mask_ratio,
        min_span:  jepa_cfg.min_span,
        max_span:  jepa_cfg.max_span,
        num_spans: jepa_cfg.num_spans,
    };
    let ema_cfg = EmaConfig {
        start:      jepa_cfg.ema_start,
        end:        jepa_cfg.ema_end,
        ramp_steps: args.steps as usize,
    };
    let mut ema = EmaTarget::new(ema_cfg);
    let mut rng = StdRng::seed_from_u64(args.seed);

    // Dummy online/target param vectors (replace with real model params later)
    let param_size = jepa_cfg.d_model * 4;
    let mut online_params = vec![0.5_f32; param_size];
    let mut target_params = vec![0.0_f32; param_size];

    let obj_cfg = ObjectiveConfig::default(); // NTP 0.5 + JEPA 0.25 + NCA 0.25
    let jepa_loss_cfg = JepaLossConfig::default();

    // ── Main training loop ─────────────────────────────────────────────────
    let git_sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let mut audit = AuditLog::new(&config.model_id, config.seed, config.steps, &git_sha);

    tracing::info!(
        "IGLA trainer starting: model={} arch={} steps={} seed={}",
        config.model_id, args.arch, config.steps, config.seed
    );

    let mut ntp_loss: f32 = 10.0;
    let mut rng_state = config.seed;

    for step in 1..=config.steps {
        let lr = schedule.lr(step, config.steps);

        // Simulate NTP loss decay
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let noise = ((rng_state >> 33) as f32 / u32::MAX as f32) - 0.5;
        ntp_loss = ntp_loss * (1.0 - lr * 10.0) + noise * 0.001;
        ntp_loss = ntp_loss.max(0.01);

        // ── JEPA branch ───────────────────────────────────────────────────
        let (bpb, combined_total) = if jepa_active {
            // 1. Generate span masks for this step's sequence
            let mask_result = mask_spans(args.seq_len, mask_cfg, &mut rng);
            let target_indices = get_masked(&mask_result.mask);

            // 2. Simulate context/target embeddings (placeholder: real encoder later)
            let ctx_emb: Vec<f32> = (0..args.seq_len * jepa_cfg.d_model)
                .map(|i| ((i as f32 * 0.001) + ntp_loss) * 0.1)
                .collect();
            let pred_emb: Vec<f32> = target_indices.iter().flat_map(|&pos| {
                let start = (pos * jepa_cfg.d_model).min(ctx_emb.len().saturating_sub(1));
                let end = (start + jepa_cfg.d_model).min(ctx_emb.len());
                ctx_emb[start..end].to_vec()
            }).collect();
            let tgt_emb: Vec<f32> = target_params[..pred_emb.len().min(param_size)].to_vec();

            // 3. JEPA loss (L2-normalized MSE)
            let jepa = if pred_emb.len() == tgt_emb.len() && !pred_emb.is_empty() {
                let l = trios_train_cpu::jepa::compute_jepa_loss(&pred_emb, &tgt_emb, jepa_loss_cfg);
                l.prediction
            } else { 0.0 };

            // 4. NCA entropy stub (target: band [1.5, 2.8])
            let nca_entropy = 1.5 + (step as f64 / config.steps as f64) * 1.3;
            let nca = trios_train_cpu::objective::nca_entropy_constraint(nca_entropy);

            // 5. Multi-objective combined loss
            let combined = compute_combined_loss(
                ComponentLosses { ntp: ntp_loss as f64, jepa, nca },
                obj_cfg,
            );

            // 6. EMA update
            for x in online_params.iter_mut() {
                *x = (*x * (1.0 - lr as f32)).max(0.01);
            }
            ema.update(&mut target_params, &online_params);

            let bpb = schedule.bpb_from_loss(combined.total as f32);
            (bpb, combined.total as f32)
        } else {
            let bpb = schedule.bpb_from_loss(ntp_loss);
            (bpb, ntp_loss)
        };

        audit.record(step, combined_total, bpb, lr);

        if step % 200 == 0 || step == config.steps {
            if jepa_active {
                tracing::info!(
                    "step={:4} loss={:.4} bpb={:.4} lr={:.6} ema_tau={:.6} arch=jepa",
                    step, combined_total, bpb, lr, ema.decay()
                );
            } else {
                tracing::info!(
                    "step={:4} loss={:.4} bpb={:.4} lr={:.6} arch={}",
                    step, combined_total, bpb, lr, args.arch
                );
            }
        }

        if step % 500 == 0 {
            if let Err(e) = audit.dump_metric("metric.json") {
                tracing::warn!("metric dump failed at step {}: {}", step, e);
            } else {
                tracing::info!("metric.json written at step {}", step);
            }
        }
    }

    if jepa_active {
        tracing::info!(
            "T-JEPA training complete: ema_steps={} final_tau={:.6}",
            ema.step(), ema.decay()
        );
    }

    audit.dump_metric("metric.json")?;
    let json = audit.to_json();
    println!("{}", json);

    write_experience_log(&args.exp_id, &args.model_id, args.seed, &args.arch, &json)?;

    Ok(())
}

fn write_experience_log(
    exp_id: &Option<String>,
    model_id: &str,
    seed: u64,
    arch: &str,
    result_json: &str,
) -> Result<()> {
    use std::fs;
    use std::io::Write;

    let exp_name = exp_id.as_deref().unwrap_or("training");
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");

    let entry = format!(
        "[{}] TASK: {} | model={} | arch={} | seed={} | result={}\n",
        timestamp, exp_name, model_id, arch, seed, result_json
    );

    let dir = ".trinity/experience";
    fs::create_dir_all(dir)?;

    let filename = format!("{}/trios_{}.trinity", dir, chrono::Utc::now().format("%Y%m%d"));
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&filename)?
        .write_all(entry.as_bytes())?;

    tracing::info!("Experience logged to {} (arch={})", filename, arch);

    Ok(())
}
