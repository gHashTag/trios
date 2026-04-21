//! IGLA-STACK-502 Integration Runner
//!
//! Target: BPB ≤ 1.12 (baseline: 1.2244 BPB)
//! Expected ΔBPB: -0.28 → Final ~0.94 BPB

use anyhow::Result;
use trios_training::{
    igla_stack_training::{IGLAStackConfig, IGLAStackTrainer},
};

fn main() -> Result<()> {
    println!("═══════════════════════════════════");
    println!("IGLA-STACK-502 Training");
    println!("═════════════════════════════════════");
    println!();

    let config = IGLAStackConfig {
        vocab_size: 256,              // TinyShakespeare
        d_model: 256,
        n_layers: 5,
        n_heads: 8,
        d_ffn: 1024,
        batch_size: 64,
        seq_len: 128,
        iterations: 20000,
        val_every: 100,

        // FOXTROT: BigramHash(729)
        bigram_vocab: 729,
        bigram_dim: 128,
        use_smear: true,

        // GOLF
        use_phi_schedule: true,
        use_sw: false,
        use_sliding_eval: true,

        // ALFA: Muon
        use_muon: true,
        muon_wd: 0.04,
    };

    println!("Configuration:");
    println!("  vocab_size: {}", config.vocab_size);
    println!("  d_model: {}", config.d_model);
    println!("  n_layers: {}", config.n_layers);
    println!("  n_heads: {}", config.n_heads);
    println!("  d_ffn: {}", config.d_ffn);
    println!("  batch_size: {}", config.batch_size);
    println!("  seq_len: {}", config.seq_len);
    println!("  iterations: {}", config.iterations);
    println!();
    println!("Techniques:");
    println!("  FOXTROT: BigramHash({}), SmearGate: {}",
        config.bigram_vocab, config.use_smear);
    println!("  GOLF: Phi-Schedule: {}, SWA: {}, Sliding Eval: {}",
        config.use_phi_schedule, config.use_sw, config.use_sliding_eval);
    println!("  ALFA: Muon optimizer: {}, WD: {}",
        config.use_muon, config.muon_wd);
    println!();

    println!("Target: BPB ≤ 1.12 (baseline: 1.2244 BPB)");
    println!("Expected ΔBPB: -0.28 → Final ~0.94 BPB");
    println!();

    // Create trainer
    println!("Creating trainer...");
    let mut trainer = IGLAStackTrainer::new(config.clone())?;

    // Run training
    println!("Starting training...");
    let final_bpb = trainer.train(&mut trainer)?;

    println!();
    println!("═══════════════════════════════════");
    println!("Final Results");
    println!("═══════════════════════════════════");
    println!();
    println!("Final BPB: {:.4}", final_bpb);

    let target_met = final_bpb <= 1.12;
    println!("Target Met (BPB ≤ 1.12): {}",
        if target_met { "✅ YES" } else { "❌ NO" });

    println!();
    if target_met {
        println!("🎉 IGLA-STACK-502 SUCCESS: BPB target achieved!");
        println!("   Ready for NeurIPS 2026 submission");
    } else {
        println!("⚠️  IGLA-STACK-502 did not meet target");
    }

    Ok(())
}
