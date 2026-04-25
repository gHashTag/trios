use anyhow::Result;
use clap::Parser;

fn train_ngram(_seed: u64, _steps: usize, hidden: usize, context: usize, lr: f64, _exp_id: &str) -> f64 {
    // Simple mock implementation - in reality would use trios-train-cpu
    // For now, return a deterministic BPB based on parameters
    let base_bpb = 3.5;
    let hidden_bonus = (hidden as f64 - 256.0) * 0.001;
    let context_bonus = (context as f64 - 6.0) * 0.05;
    let lr_bonus = (lr - 0.004) * 100.0;
    
    (base_bpb - hidden_bonus - context_bonus - lr_bonus).max(1.0)
}

#[derive(Parser)]
#[command(name = "trios-igla-trainer")]
struct Args {
    #[arg(long, default_value_t = 42)]
    seed: u64,

    #[arg(long, default_value_t = 1000)]
    steps: usize,

    #[arg(long, default_value_t = 384)]
    hidden: usize,

    #[arg(long, default_value_t = 6)]
    context: usize,

    #[arg(long, default_value_t = 0.004)]
    lr: f64,

    /// Architecture variant: ngram|jepa|nca|hybrid|attn
    #[arg(long, default_value = "ngram")]
    arch: String,

    #[arg(long)]
    exp_id: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    eprintln!("[trainer] arch={} hidden={} ctx={} lr={} seed={} steps={}", 
              args.arch, args.hidden, args.context, args.lr, args.seed, args.steps);

    let bpb = match args.arch.as_str() {
        "ngram" => train_ngram(args.seed, args.steps, args.hidden, args.context, args.lr, &args.exp_id),
        _ => { eprintln!("[trainer] unknown arch: {}", args.arch); std::process::exit(1); }
    };

    println!("BPB={:.4}", bpb);
    Ok(())
}