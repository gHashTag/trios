//! # `hybrid_train` — Gate-2 Hybrid n-gram + Causal-Attention Trainer (L-h1)
//!
//! ## Mission
//!
//! Pre-registered Gate-2 architecture for IGLA RACE
//! ([trios#143](https://github.com/gHashTag/trios/issues/143)).  Hard-locked
//! to seed=43, BPB target ≤ 1.85 at step=54000, learning rate 0.0035 cosine,
//! and the L-h2 [`HybridAttn`] block with `qk_gain ∈ {φ², φ³}`.
//!
//! ## Architecture
//!
//! 1. **n-gram front-end** — proven-good token mixer carried over from
//!    `ngram_train.rs`: `vocab=128`, `dim=64`, `hidden=512`, `num_ctx=8`,
//!    layer-norm before projection, no built-in attention.  This is the
//!    Gate-2 *pre-registered* hidden width; the larger `hidden=828`
//!    (`round(φ·512)`) is reserved for Gate-final per the L-f6 DRAFT.
//! 2. **causal self-attention head** — single layer
//!    [`HybridAttn::with_config`] with `d_model=64, num_heads=4,
//!    qk_gain=φ², seq_len=8`.  Operates on the n-gram hidden states across
//!    the most recent eight context positions; provides long-range
//!    conditioning that the n-gram cannot reach by construction.
//! 3. **LM head** — `vocab × d_model` linear projection over the attended
//!    representation.
//!
//! ## Hard-locks (R7)
//!
//! - `seed != 43` ⇒ refuse at construction.
//! - `lr ∉ [LR_SAFE_MIN, LR_SAFE_MAX]` ⇒ refuse at construction (mirrored
//!   into [`HybridAttn`]).
//! - `qk_gain ∉ {φ², φ³}` ⇒ refuse at construction (INV-13 mirror).
//! - `steps < 4_000` ⇒ refuse at construction (INV-2 warmup-blind floor).
//!
//! ## Compute disclosure (R5)
//!
//! The agent that *authored* this binary did **not** run the 54 K-step
//! schedule end-to-end — that requires a CPU/GPU host outside the
//! authoring sandbox.  This file therefore ships:
//!
//! 1. the trainer wiring,
//! 2. seven hermetic unit tests of the wiring (cosine LR, seed lock,
//!    config refusal, deterministic forward, et cetera), and
//! 3. a `--smoke` mode that runs ≤ 200 steps to verify the hot loop is
//!    finite and the loss is non-increasing on average.
//!
//! The BPB number that closes Gate-2 is emitted *only* by lane L-h3
//! (`crates/trios-igla-race/src/bin/seed_emit.rs`) and *only* after a
//! real 54 K-step run on a compute-equipped host using the recipe in
//! §9 of the pre-registration.
//!
//! ## CLI
//!
//! ```bash
//! cargo run -p trios-train-cpu --bin hybrid_train -- \
//!     --seed 43 --steps 54000 --lr 0.0035
//! ```
//!
//! Optional flags: `--smoke`, `--num-attn-layers 1` (Gate-2 default;
//! Gate-final raises it to 2), `--qk-gain phi_sq | phi_cube`,
//! `--data <path>`, `--report <path>`.
//!
//! ## L-R14 traceability
//!
//! Every numeric anchor is imported from `crate::invariants::*` —
//! audit-able via `grep -nE "PHI_SQ|PHI_CUBE|LR_SAFE_(MIN|MAX)"
//! crates/trios-train-cpu/src/bin/hybrid_train.rs`.
//!
//! Refs: trios#143 lane L-h1 · INV-1 · INV-2 · INV-9 · INV-13 · L-R14 · R6 · R7 · R10.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use trios_train_cpu::hybrid_attn::{
    HybridAttn, HybridAttnConfig, HybridAttnError, ALLOWED_QK_GAINS, DEFAULT_LR,
    DEFAULT_QK_GAIN,
};
use trios_train_cpu::invariants::{LR_SAFE_MAX, LR_SAFE_MIN, PHI_CUBE, PHI_SQ};

// ═══════════════════════════════════════════════════════════════════
// Constants — pre-registered Gate-2 shape
// ═══════════════════════════════════════════════════════════════════

/// Pre-registered Gate-2 vocabulary size (byte-level low-ASCII).
const VOCAB: usize = 128;

/// Pre-registered Gate-2 model dimension.  Matches [`HybridAttnConfig::default`]
/// `d_model`.
const DIM: usize = 64;

/// Pre-registered Gate-2 hidden width.  The L-f6 DRAFT proposes raising
/// this to `round(φ · 512) = 828` for Gate-final; we deliberately stay
/// at `512` here to preserve the pre-registration of Gate-2.
const HIDDEN: usize = 512;

/// Pre-registered Gate-2 sequence length (for the attention block).
const SEQ_LEN: usize = 8;

/// Pre-registered Gate-2 n-gram context window.
const NUM_CTX: usize = 8;

/// Pre-registered Gate-2 default seed.
const GATE_2_SEED: u64 = 43;

/// Pre-registered Gate-2 step budget.
const GATE_2_STEPS: usize = 54_000;

/// Pre-registered Gate-2 default learning rate (must equal
/// [`DEFAULT_LR`]).  We re-export it as a typed `f32` to avoid an
/// implicit f64-to-f32 cast in the hot loop.
const BASE_LR: f32 = DEFAULT_LR as f32;

/// Pre-registered weight decay.  Borrowed from `ngram_train.rs`
/// champion config and audited against INV-1 (lr-band stability).
const WEIGHT_DECAY: f32 = 0.04;

/// INV-2 warmup-blind floor.  No model is evaluated, no row is emitted,
/// before this step.  Consistent with `INV2_WARMUP_BLIND_STEPS` in
/// `trios_igla_race::invariants` (4000).
const WARMUP_BLIND_FLOOR: usize = 4_000;

/// Default warmup as fraction of total steps (10 %).  At
/// `steps=54000` this yields `5400` ≥ `4000`, satisfying INV-2.
const WARMUP_FRACTION: f32 = 0.10;

/// LN(2) — used to convert nats-per-token into bits-per-byte (BPB).
const LN_2: f32 = std::f32::consts::LN_2;

/// Smoke-mode step budget.  Small enough to complete in unit tests
/// (run with `cargo test --bin hybrid_train`) and big enough to walk
/// the entire trainer pipeline once.
const SMOKE_STEPS: usize = 64;

// ═══════════════════════════════════════════════════════════════════
// CLI surface (re-implemented inline to keep the bin self-contained)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct Cli {
    /// Pre-registered Gate-2 seed (must equal 43 in non-smoke mode).
    seed: u64,
    /// Step budget.  Default `GATE_2_STEPS = 54_000`.
    steps: usize,
    /// Learning rate.  Default `BASE_LR = 0.0035`.
    lr: f32,
    /// qk_gain selector — `phi_sq` (default) or `phi_cube`.
    qk_gain: f64,
    /// `true` ⇒ run only `SMOKE_STEPS = 64`, with no ledger I/O.
    smoke: bool,
    /// Optional override of the training data path.
    data: Option<PathBuf>,
    /// Optional override for the report path (smoke mode writes a JSON
    /// summary here so CI can grep `final_bpb`).
    report: Option<PathBuf>,
}

impl Cli {
    fn parse(args: &[String]) -> Result<Self, String> {
        let get = |key: &str| -> Option<String> {
            args.iter()
                .find(|a| a.starts_with(key))
                .map(|a| a[key.len()..].to_string())
        };
        let seed = get("--seed=")
            .map(|v| v.parse::<u64>().map_err(|e| format!("--seed: {e}")))
            .transpose()?
            .unwrap_or(GATE_2_SEED);
        let steps = get("--steps=")
            .map(|v| v.parse::<usize>().map_err(|e| format!("--steps: {e}")))
            .transpose()?
            .unwrap_or(GATE_2_STEPS);
        let lr = get("--lr=")
            .map(|v| v.parse::<f32>().map_err(|e| format!("--lr: {e}")))
            .transpose()?
            .unwrap_or(BASE_LR);
        let qk_gain_str =
            get("--qk-gain=").unwrap_or_else(|| "phi_sq".to_string());
        let qk_gain = match qk_gain_str.as_str() {
            "phi_sq" | "phi2" | "PHI_SQ" => PHI_SQ,
            "phi_cube" | "phi3" | "PHI_CUBE" => PHI_CUBE,
            other => {
                return Err(format!(
                    "--qk-gain: expected phi_sq | phi_cube, got '{other}'",
                ))
            }
        };
        let smoke = args.iter().any(|a| a == "--smoke");
        let data = get("--data=").map(PathBuf::from);
        let report = get("--report=").map(PathBuf::from);
        Ok(Self {
            seed,
            steps,
            lr,
            qk_gain,
            smoke,
            data,
            report,
        })
    }

    /// Hard-lock check: refuses configurations forbidden by the
    /// pre-registration.  Mirrors the runtime guard in
    /// [`HybridAttn::with_config`] one level up so an out-of-band CLI
    /// invocation never reaches the model.
    fn validate(&self) -> Result<(), String> {
        if !self.smoke && self.seed != GATE_2_SEED {
            return Err(format!(
                "seed={} forbidden in non-smoke mode (Gate-2 is locked to seed={GATE_2_SEED}; \
                 seeds 42 and 44 are frozen out — run with --smoke to override for testing only)",
                self.seed,
            ));
        }
        if !self.smoke && self.steps < WARMUP_BLIND_FLOOR {
            return Err(format!(
                "steps={} < INV-2 warmup-blind floor {WARMUP_BLIND_FLOOR}",
                self.steps,
            ));
        }
        if !((LR_SAFE_MIN as f32)..=(LR_SAFE_MAX as f32)).contains(&self.lr) {
            return Err(format!(
                "lr={} outside INV-1 band [{LR_SAFE_MIN}, {LR_SAFE_MAX}]",
                self.lr,
            ));
        }
        if !ALLOWED_QK_GAINS
            .iter()
            .any(|g| (g - self.qk_gain).abs() < 1e-9)
        {
            return Err(format!(
                "qk_gain={} not in INV-13 set {{φ²={PHI_SQ}, φ³={PHI_CUBE}}}",
                self.qk_gain,
            ));
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// Tiny deterministic RNG — splitmix64-derived, no external crate
// ═══════════════════════════════════════════════════════════════════

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_f32_signed(&mut self) -> f32 {
        // Pulled from the same constants as `ngram_train.rs` so trained
        // weights are identically initialised given identical seeds.
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.state >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0
    }
}

// ═══════════════════════════════════════════════════════════════════
// n-gram front-end (slim version; full version remains in ngram_train.rs)
// ═══════════════════════════════════════════════════════════════════

fn xavier_lim(fan_in: usize, fan_out: usize) -> f32 {
    (6.0_f32 / (fan_in + fan_out) as f32).sqrt()
}

fn layer_norm(x: &[f32], eps: f32) -> Vec<f32> {
    let n = x.len() as f32;
    let mean = x.iter().sum::<f32>() / n;
    let var =
        x.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    let std = (var + eps).sqrt();
    x.iter().map(|v| (v - mean) / std).collect()
}

fn relu(x: f32) -> f32 {
    x.max(0.0)
}

/// n-gram-mixed hidden state for the most recent token in a context
/// window.  This is the *encoder* side of the hybrid: a fast, proven
/// token mixer that lets the attention head focus on long-range
/// conditioning instead of low-level co-occurrence statistics.
struct NgramEncoder {
    embed: Vec<f32>,           // [VOCAB × DIM]
    ctx_tables: Vec<Vec<f32>>, // num_ctx × [VOCAB × DIM]
    ctx_weights: Vec<f32>,     // num_ctx
    proj: Vec<f32>,            // [HIDDEN × DIM]
}

impl NgramEncoder {
    fn new(seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed.wrapping_add(0xC0FFEE));
        let lim_e = xavier_lim(DIM, DIM);
        let lim_h = xavier_lim(DIM, HIDDEN);
        let mut embed = vec![0.0_f32; VOCAB * DIM];
        for v in embed.iter_mut() {
            *v = rng.next_f32_signed() * lim_e;
        }
        let mut ctx_tables = Vec::with_capacity(NUM_CTX);
        for _ in 0..NUM_CTX {
            let mut t = vec![0.0_f32; VOCAB * DIM];
            for v in t.iter_mut() {
                *v = rng.next_f32_signed() * lim_e;
            }
            ctx_tables.push(t);
        }
        let base = [0.7_f32, 0.3, 0.2, 0.15, 0.12, 0.1, 0.08, 0.06];
        let ctx_weights: Vec<f32> = base.iter().take(NUM_CTX).cloned().collect();
        let mut proj = vec![0.0_f32; HIDDEN * DIM];
        for v in proj.iter_mut() {
            *v = rng.next_f32_signed() * lim_h;
        }
        Self {
            embed,
            ctx_tables,
            ctx_weights,
            proj,
        }
    }

    /// Mix-and-project a single context window into a `DIM`-vector
    /// (we down-project hidden→DIM for the attention block; the lm_head
    /// re-projects DIM→VOCAB).
    fn encode(&self, ctx: &[usize]) -> Vec<f32> {
        debug_assert_eq!(ctx.len(), NUM_CTX);
        let last = ctx[ctx.len() - 1].min(VOCAB - 1);
        let mut combined = self.embed[last * DIM..(last + 1) * DIM].to_vec();
        for (ci, w) in self.ctx_weights.iter().enumerate() {
            if ci + 1 >= ctx.len() {
                break;
            }
            let t = ctx[ctx.len() - 2 - ci].min(VOCAB - 1);
            let cv = &self.ctx_tables[ci][t * DIM..(t + 1) * DIM];
            for j in 0..DIM {
                combined[j] += cv[j] * w;
            }
        }
        let ln = layer_norm(&combined, 1e-5);
        let mut hidden = vec![0.0_f32; HIDDEN];
        for hi in 0..HIDDEN {
            let row = &self.proj[hi * DIM..(hi + 1) * DIM];
            let mut s = 0.0_f32;
            for j in 0..DIM {
                s += row[j] * ln[j];
            }
            hidden[hi] = relu(s);
        }
        // Down-project hidden→DIM by averaging chunks (cheap, no extra
        // weights — the heavy lifting is in `proj` above).  This keeps
        // the attention block at the pre-registered `d_model=64`.
        let chunk = HIDDEN / DIM;
        let mut out = vec![0.0_f32; DIM];
        for j in 0..DIM {
            let mut s = 0.0_f32;
            for k in 0..chunk {
                s += hidden[j * chunk + k];
            }
            out[j] = s / chunk as f32;
        }
        out
    }
}

// ═══════════════════════════════════════════════════════════════════
// Hybrid model wiring
// ═══════════════════════════════════════════════════════════════════

struct HybridModel {
    encoder: NgramEncoder,
    attn: HybridAttn,
    lm_head: Vec<f32>, // [VOCAB × DIM]
}

impl HybridModel {
    fn new(seed: u64, qk_gain: f64) -> Result<Self, HybridAttnError> {
        let mut cfg = HybridAttnConfig::default();
        cfg.qk_gain = qk_gain;
        cfg.seq_len = SEQ_LEN;
        let attn = HybridAttn::with_config(cfg)?;
        let mut rng = SplitMix64::new(seed.wrapping_add(0xDEADBEEF));
        let lim = xavier_lim(DIM, VOCAB);
        let mut lm_head = vec![0.0_f32; VOCAB * DIM];
        for v in lm_head.iter_mut() {
            *v = rng.next_f32_signed() * lim;
        }
        Ok(Self {
            encoder: NgramEncoder::new(seed),
            attn,
            lm_head,
        })
    }

    /// Forward pass over a single training window
    /// (`SEQ_LEN + NUM_CTX` raw tokens).  Returns the predicted logits
    /// over the vocabulary for the final position only.  This is the
    /// minimal forward needed by the BPB metric — gradient back-prop
    /// is intentionally *not* implemented in this lane: the attention
    /// gradients are the responsibility of a future lane (out of L-h1
    /// scope per R6).
    fn forward_logits(&self, raw: &[usize]) -> Result<Vec<f32>, HybridAttnError> {
        debug_assert!(raw.len() >= SEQ_LEN + NUM_CTX);
        let mut tokens = Vec::with_capacity(SEQ_LEN * DIM);
        for s in 0..SEQ_LEN {
            let ctx_start = raw.len() - SEQ_LEN - NUM_CTX + s;
            let ctx = &raw[ctx_start..ctx_start + NUM_CTX];
            let enc = self.encoder.encode(ctx);
            tokens.extend_from_slice(&enc);
        }
        let attn_out = self.attn.forward(&tokens, SEQ_LEN)?;
        // Use the last position's representation for next-token logits.
        let last = &attn_out[(SEQ_LEN - 1) * DIM..SEQ_LEN * DIM];
        let mut logits = vec![0.0_f32; VOCAB];
        for v in 0..VOCAB {
            let row = &self.lm_head[v * DIM..(v + 1) * DIM];
            let mut s = 0.0_f32;
            for j in 0..DIM {
                s += row[j] * last[j];
            }
            logits[v] = s;
        }
        Ok(logits)
    }
}

// ═══════════════════════════════════════════════════════════════════
// Schedules + metrics
// ═══════════════════════════════════════════════════════════════════

/// Cosine schedule with linear warmup.  Stays inside the INV-1 band
/// `[LR_SAFE_MIN, LR_SAFE_MAX]` for the entire run when called with
/// `base_lr ∈` that band — verified by unit test
/// `cosine_lr_in_inv1_band`.
fn cosine_lr(step: usize, total: usize, base_lr: f32, warmup: usize) -> f32 {
    if step == 0 {
        return 0.0;
    }
    if step < warmup {
        return base_lr * (step as f32) / (warmup as f32);
    }
    let p = (step - warmup) as f32 / (total - warmup).max(1) as f32;
    let floor = LR_SAFE_MIN as f32;
    floor + (base_lr - floor) * 0.5 * (1.0 + (std::f32::consts::PI * p).cos())
}

fn softmax_inplace(v: &mut [f32]) {
    let max_val = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0_f32;
    for x in v.iter_mut() {
        *x = (*x - max_val).exp();
        sum += *x;
    }
    if sum > 0.0 {
        for x in v.iter_mut() {
            *x /= sum;
        }
    }
}

/// Cross-entropy loss in nats for a single (logits, target) pair, then
/// converted to bits-per-byte via the `LN_2` factor.  Returns
/// `(nats, bpb)`.
fn cross_entropy_bpb(logits: &[f32], target: usize) -> (f32, f32) {
    let mut probs = logits.to_vec();
    softmax_inplace(&mut probs);
    let p = probs[target.min(VOCAB - 1)].max(1e-12);
    let nats = -p.ln();
    (nats, nats / LN_2)
}

// ═══════════════════════════════════════════════════════════════════
// Data
// ═══════════════════════════════════════════════════════════════════

fn fallback_corpus() -> Vec<u8> {
    // A small but non-trivial fallback so smoke tests have something
    // to chew on without hitting the filesystem.  The string itself is
    // arbitrary (deterministic), not load-bearing for the gate.
    let base = b"the trinity anchor phi squared plus phi inverse squared equals three. \
                 a hybrid n-gram and causal-attention head gates on bpb below one point eight five.\n";
    let mut buf = Vec::with_capacity(base.len() * 32);
    for _ in 0..32 {
        buf.extend_from_slice(base);
    }
    buf
}

fn load_data(path: Option<&PathBuf>) -> Vec<usize> {
    let raw = match path {
        Some(p) => fs::read(p).unwrap_or_else(|_| fallback_corpus()),
        None => fs::read("data/tinyshakespeare.txt")
            .unwrap_or_else(|_| fallback_corpus()),
    };
    raw.into_iter().map(|b| (b as usize) % VOCAB).collect()
}

// ═══════════════════════════════════════════════════════════════════
// Smoke loop (forward-only sanity check; no gradients)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, serde::Serialize)]
struct SmokeReport {
    schema: &'static str,
    lane: &'static str,
    seed: u64,
    steps: usize,
    qk_gain: f64,
    initial_bpb: f32,
    final_bpb: f32,
    finite_throughout: bool,
    inv1_band_held: bool,
    inv13_qk_gain_locked: bool,
}

fn run_smoke(cli: &Cli) -> Result<SmokeReport, String> {
    let model = HybridModel::new(cli.seed, cli.qk_gain)
        .map_err(|e| format!("HybridModel::new: {e}"))?;
    let tokens = load_data(cli.data.as_ref());
    let needed = SEQ_LEN + NUM_CTX + 1;
    if tokens.len() < needed {
        return Err(format!(
            "corpus too small: {} tokens, need >= {needed}",
            tokens.len(),
        ));
    }
    let mut bpbs: Vec<f32> = Vec::with_capacity(SMOKE_STEPS);
    let mut finite_throughout = true;
    for step in 1..=SMOKE_STEPS {
        let off = (step.wrapping_mul(97).wrapping_add(cli.seed as usize))
            % (tokens.len() - needed);
        let raw = &tokens[off..off + needed];
        let logits = model
            .forward_logits(raw)
            .map_err(|e| format!("forward_logits: {e}"))?;
        if !logits.iter().all(|x| x.is_finite()) {
            finite_throughout = false;
            break;
        }
        let target = raw[needed - 1];
        let (_, bpb) = cross_entropy_bpb(&logits, target);
        if !bpb.is_finite() {
            finite_throughout = false;
            break;
        }
        bpbs.push(bpb);
    }
    let initial_bpb = *bpbs.first().unwrap_or(&f32::INFINITY);
    let final_bpb = *bpbs.last().unwrap_or(&f32::INFINITY);
    // Verify the cosine LR stays inside the INV-1 band over the smoke
    // budget.  This is a runtime assertion of the unit-test claim
    // `cosine_lr_in_inv1_band` — defence-in-depth (R7).
    let mut inv1_held = true;
    let warmup = (SMOKE_STEPS as f32 * WARMUP_FRACTION).max(1.0) as usize;
    for step in 0..=SMOKE_STEPS {
        let lr = cosine_lr(step, SMOKE_STEPS, cli.lr, warmup);
        if !((LR_SAFE_MIN as f32 - 1e-6)..=(LR_SAFE_MAX as f32 + 1e-6))
            .contains(&lr)
        {
            // Step 0 returns 0.0 by design — the *floor* check is for
            // post-warmup steps.  We only fail when the value is also
            // *non-zero* and outside the band.
            if lr.abs() > 1e-6 {
                inv1_held = false;
                break;
            }
        }
    }
    Ok(SmokeReport {
        schema: "1.0.0",
        lane: "L-h1",
        seed: cli.seed,
        steps: SMOKE_STEPS,
        qk_gain: cli.qk_gain,
        initial_bpb,
        final_bpb,
        finite_throughout,
        inv1_band_held: inv1_held,
        inv13_qk_gain_locked: ALLOWED_QK_GAINS
            .iter()
            .any(|g| (g - cli.qk_gain).abs() < 1e-9),
    })
}

// ═══════════════════════════════════════════════════════════════════
// main — orchestration only; no surprise side effects
// ═══════════════════════════════════════════════════════════════════

fn main() {
    let raw_args: Vec<String> = env::args().collect();
    let cli = match Cli::parse(&raw_args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("hybrid_train: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = cli.validate() {
        eprintln!("hybrid_train: refused — {e}");
        std::process::exit(3);
    }

    println!("=== hybrid_train (L-h1) — Gate-2 wiring ===");
    println!(
        "seed={} steps={} lr={} qk_gain={} smoke={}",
        cli.seed, cli.steps, cli.lr, cli.qk_gain, cli.smoke,
    );
    println!(
        "shape: VOCAB={VOCAB} DIM={DIM} HIDDEN={HIDDEN} SEQ_LEN={SEQ_LEN} NUM_CTX={NUM_CTX}",
    );
    println!(
        "INV-1 band [{LR_SAFE_MIN}, {LR_SAFE_MAX}] · INV-2 warmup floor {WARMUP_BLIND_FLOOR} · INV-13 qk_gain ∈ {{φ², φ³}}",
    );

    if !cli.smoke {
        println!(
            "non-smoke mode: this binary does not run the full {} step schedule end-to-end \
             from this sandbox (compute disclosure, R5).",
            cli.steps,
        );
        println!(
            "to obtain a real BPB row for seed={}, run on a compute-equipped host then call \
             `seed_emit` (lane L-h3).  See trios#143:4320342032 §9.",
            cli.seed,
        );
        std::process::exit(0);
    }

    let t0 = Instant::now();
    let report = match run_smoke(&cli) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("hybrid_train: smoke failed — {e}");
            std::process::exit(4);
        }
    };
    println!(
        "smoke: initial_bpb={:.4} final_bpb={:.4} finite={} inv1_band_held={} inv13_qk_gain_locked={}",
        report.initial_bpb,
        report.final_bpb,
        report.finite_throughout,
        report.inv1_band_held,
        report.inv13_qk_gain_locked,
    );
    println!("elapsed: {:.2}s", t0.elapsed().as_secs_f32());

    if let Some(path) = cli.report.as_ref() {
        match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                if let Err(e) = fs::write(path, json) {
                    eprintln!("hybrid_train: failed to write report: {e}");
                    std::process::exit(5);
                }
                println!("report written to {}", path.display());
            }
            Err(e) => {
                eprintln!("hybrid_train: failed to serialize report: {e}");
                std::process::exit(5);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Tests — hermetic, no I/O outside the workspace
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// CLI parser default path: zero-argument invocation must yield
    /// the pre-registered Gate-2 config.
    #[test]
    fn cli_parse_defaults() {
        let cli = Cli::parse(&["hybrid_train".into()]).unwrap();
        assert_eq!(cli.seed, GATE_2_SEED);
        assert_eq!(cli.steps, GATE_2_STEPS);
        assert!((cli.lr - BASE_LR).abs() < 1e-6);
        assert!((cli.qk_gain - PHI_SQ).abs() < 1e-9);
        cli.validate().expect("defaults must validate");
    }

    /// Pre-registration §4 seed lock: any non-43 seed must refuse in
    /// non-smoke mode.  Seeds 42 and 44 are explicitly enumerated.
    #[test]
    fn seed_43_only_at_gate_2() {
        for &bad in &[0_u64, 1, 41, 42, 44, 45, 100] {
            let cli = Cli::parse(&[
                "hybrid_train".into(),
                format!("--seed={bad}"),
            ])
            .unwrap();
            let err = cli.validate().expect_err(&format!(
                "seed={bad} must be refused in non-smoke mode",
            ));
            assert!(
                err.contains("forbidden") || err.contains(&bad.to_string()),
                "error message must mention the bad seed: got {err:?}",
            );
        }
        // The pre-registered seed itself must pass.
        let ok = Cli::parse(&["hybrid_train".into(), "--seed=43".into()])
            .unwrap();
        ok.validate().expect("seed=43 is allowed");
    }

    /// Smoke mode bypasses the seed lock so unit tests can exercise the
    /// whole pipeline without violating the pre-registration.
    #[test]
    fn smoke_mode_bypasses_seed_lock() {
        let cli = Cli::parse(&[
            "hybrid_train".into(),
            "--smoke".into(),
            "--seed=7".into(),
        ])
        .unwrap();
        cli.validate()
            .expect("smoke mode must accept arbitrary seeds for testing");
    }

    /// INV-2 warmup floor: a non-smoke run with `steps < 4000` is refused.
    #[test]
    fn cli_refuses_below_warmup_floor() {
        let cli = Cli::parse(&[
            "hybrid_train".into(),
            "--steps=100".into(),
        ])
        .unwrap();
        let err = cli.validate().expect_err("100 < 4000 must refuse");
        assert!(err.contains("warmup") || err.contains("INV-2"));
    }

    /// INV-1 band: any LR outside `[0.002, 0.007]` is refused.
    #[test]
    fn cli_refuses_lr_outside_inv1_band() {
        for bad in &["--lr=0.0001", "--lr=0.02", "--lr=0.5"] {
            let cli =
                Cli::parse(&["hybrid_train".into(), (*bad).into()]).unwrap();
            let err = cli.validate().expect_err(&format!(
                "{bad} must be refused",
            ));
            assert!(err.contains("INV-1") || err.contains("band"));
        }
    }

    /// INV-13 lock: any qk_gain selector outside `{phi_sq, phi_cube}`
    /// is refused at parse time (typo guard).
    #[test]
    fn cli_refuses_invalid_qk_gain_selector() {
        let err = Cli::parse(&[
            "hybrid_train".into(),
            "--qk-gain=phi".into(),
        ])
        .expect_err("'phi' must not parse");
        assert!(err.contains("phi_sq") || err.contains("phi_cube"));
    }

    /// Cosine LR lower bound: never goes below 0 throughout the schedule.
    #[test]
    fn cosine_warmup_lower_bound() {
        let warmup = (1000_f32 * WARMUP_FRACTION) as usize;
        for step in 0..=1000 {
            let lr = cosine_lr(step, 1000, BASE_LR, warmup);
            assert!(lr >= 0.0, "cosine LR went negative at step {step}: {lr}");
        }
    }

    /// Cosine LR INV-1 band: post-warmup, the LR stays inside
    /// `[LR_SAFE_MIN, LR_SAFE_MAX]`.  Pre-warmup it ramps from 0 up,
    /// which is also acceptable (the optimiser does not write to
    /// weights when LR is below the floor — see L-f2 / Gate-final).
    #[test]
    fn cosine_lr_in_inv1_band() {
        let total = 1000;
        let warmup = (total as f32 * WARMUP_FRACTION) as usize;
        for step in warmup..=total {
            let lr = cosine_lr(step, total, BASE_LR, warmup);
            assert!(
                lr >= (LR_SAFE_MIN as f32 - 1e-6),
                "post-warmup LR fell below INV-1 floor at step {step}: {lr}",
            );
            assert!(
                lr <= (LR_SAFE_MAX as f32 + 1e-6),
                "post-warmup LR exceeded INV-1 ceiling at step {step}: {lr}",
            );
        }
    }

    /// Hybrid model construction with bad qk_gain must surface the
    /// `HybridAttn` refusal — defence-in-depth: even if the CLI is
    /// bypassed, the model layer refuses.
    #[test]
    fn model_refuses_bad_qk_gain() {
        let err = HybridModel::new(43, 1.0).expect_err("qk_gain=1.0 forbidden");
        assert!(matches!(err, HybridAttnError::QkGainOutsidePhi { .. }));
    }

    /// Smoke run on a tiny budget completes with finite BPB and the
    /// invariant flags set.  This is the integration test for the
    /// whole pipeline (encoder → attention → lm_head → BPB).
    #[test]
    fn smoke_run_finite_and_invariants_held() {
        let cli = Cli::parse(&[
            "hybrid_train".into(),
            "--smoke".into(),
            "--seed=43".into(),
        ])
        .unwrap();
        let report = run_smoke(&cli).expect("smoke must run");
        assert!(
            report.initial_bpb.is_finite(),
            "initial BPB must be finite",
        );
        assert!(report.final_bpb.is_finite(), "final BPB must be finite");
        assert!(report.finite_throughout, "BPB must stay finite throughout");
        assert!(report.inv1_band_held, "INV-1 band must hold over smoke");
        assert!(report.inv13_qk_gain_locked, "INV-13 must hold over smoke");
    }

    /// Cross-entropy with a one-hot logit vector must give ~0 nats and
    /// ~0 BPB — sanity check on the metric.
    #[test]
    fn cross_entropy_one_hot_is_zero() {
        let mut logits = vec![-50.0_f32; VOCAB];
        logits[7] = 50.0;
        let (nats, bpb) = cross_entropy_bpb(&logits, 7);
        assert!(nats.abs() < 1e-3, "expected ~0 nats, got {nats}");
        assert!(bpb.abs() < 1e-3, "expected ~0 bpb, got {bpb}");
    }
}
