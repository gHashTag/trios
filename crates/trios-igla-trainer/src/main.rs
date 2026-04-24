use anyhow::Result;
use clap::Parser;

mod jepa_runner;

#[derive(Parser)]
#[command(name = "igla-trainer")]
struct Args {
    #[arg(long, default_value = "ngram")]
    arch: String,

    #[arg(long, default_value_t = 384)]
    hidden: usize,

    #[arg(long, default_value_t = 6)]
    context: usize,

    #[arg(long, default_value_t = 0.004)]
    lr: f64,

    #[arg(long, default_value_t = 1000)]
    steps: usize,

    #[arg(long, default_value_t = 42)]
    seed: u64,

    #[arg(long)]
    exp_id: Option<String>,
}

/// Mock training simulation for IGLA RACE
/// Returns BPB based on hyperparameters (simulates convergence)
fn simulate_training(config: &Args) -> f64 {
    // Base BPB that decreases with:
    // - More steps
    // - Larger hidden dimension
    // - Optimal context (~6)
    // - Optimal LR (~0.004)

    let base_bpb = 3.5;

    // Hidden dim benefit: larger = better (diminishing returns)
    let hidden_benefit = ((config.hidden as f64).log2() - 7.0) * 0.3;

    // Context penalty: too small or too large hurts
    let ctx_diff = (config.context as f64 - 6.0).abs();
    let context_penalty = ctx_diff * 0.15;

    // LR penalty: far from 0.004 hurts
    let lr_diff = (config.lr - 0.004).abs() / 0.004;
    let lr_penalty = lr_diff * 0.2;

    // Steps benefit: more steps = better (logarithmic)
    let steps_benefit = ((config.steps as f64) / 1000.0).ln() * 0.4;

    // Architecture base (mock values)
    let arch_base = match config.arch.as_str() {
        "ngram" => 0.0,
        "attn" => -0.1,   // Slightly better theoretically
        "hybrid" => -0.05,
        "jepa" => -0.15,   // Joint embedding — best theoretical
        _ => 0.0,
    };

    // Random noise based on seed
    let mut rng_state = config.seed;
    rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let noise = ((rng_state >> 33) as f64 / u32::MAX as f64 - 0.5) * 0.05;

    let final_bpb = base_bpb
        + hidden_benefit
        + context_penalty
        + lr_penalty
        - steps_benefit
        + arch_base
        + noise;

    final_bpb.max(1.2) // Floor at 1.2 (won't reach IGLA target without real training)
}

fn main() -> Result<()> {
    // Initialize logging to stderr only
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    eprintln!(
        "IGLA trainer: arch={} hidden={} context={} lr={} steps={} seed={}",
        args.arch, args.hidden, args.context, args.lr, args.steps, args.seed
    );

    // Dispatch by architecture
    let bpb = match args.arch.as_str() {
        "jepa" => {
            use trios_train_cpu::jepa::JepaConfig;

            let cfg = JepaConfig {
                seed: args.seed,
                d_model: args.hidden,
                mask_ratio: 0.30,
                min_span: 3,
                max_span: 7,
                num_spans: 2,
                ema_start: 0.996,
                ema_end: 1.0,
                ema_ramp_steps: args.steps,
                predictor_lr_mult: 0.1,
            };

            let train_args = jepa_runner::JepaTrainArgs::from_clap(&args);
            jepa_runner::run_jepa_training(&cfg, &train_args)?
        }
        _ => {
            // Original mock simulation for ngram, attn, hybrid
            for step in (0..args.steps).step_by(100) {
                eprintln!("Step {} / {}", step, args.steps);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            simulate_training(&args)
        }
    };

    // stdout: ONLY BPB=X.XXXX (contract with asha.rs)
    println!("BPB={:.4}", bpb);

    Ok(())
}

#[allow(dead_code)]
fn write_experience_log(exp_id: &str, config: &Args, bpb: f64) -> Result<()> {
    use std::fs;
    use std::io::Write;

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let entry = format!(
        "[{}] {} | arch={} h={} ctx={} lr={} steps={} seed={} | BPB={:.4}\n",
        timestamp,
        exp_id,
        config.arch,
        config.hidden,
        config.context,
        config.lr,
        config.steps,
        config.seed,
        bpb
    );

    let dir = ".trinity/experience";
    fs::create_dir_all(dir)?;

    let filename = format!("{}/trios_{}.trinity", dir, chrono::Utc::now().format("%Y%m%d"));
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&filename)?
        .write_all(entry.as_bytes())?;

    eprintln!("Experience logged to {}", filename);

    Ok(())
}
