//! Gradient computation for T-JEPA training.
//!
//! Module: gradient
//! Size: ≤ 60 lines (NASA Rule 4)

use crate::model::NgramModel;

const DIM: usize = 64;
const HIDDEN: usize = 384;
const NUM_CTX: usize = 4;
const NGRAM: usize = NUM_CTX + 2;
const VOCAB: usize = 128;
const LN_2: f32 = std::f32::consts::LN_2;

/// Gradient accumulators for model parameters.
pub struct TrainGrads {
    pub g_embed: Vec<f32>,
    pub g_ctx: Vec<Vec<f32>>,
    pub g_proj: Vec<f32>,
    pub g_head: Vec<f32>,
}

/// Compute gradients for a training sequence.
///
/// # Arguments
/// - `model`: Current model parameters
/// - `tokens`: Input token sequence
///
/// # Returns
/// Tuple of (gradients, hidden states, NTP loss)
pub fn compute_grads(
    model: &NgramModel,
    tokens: &[usize],
) -> (TrainGrads, Vec<Vec<f32>>, f32) {
    let count = tokens.len().saturating_sub(NGRAM);
    assert!(count > 0, "sequence too short for gradient computation");

    let mut g_embed = vec![0.0f32; VOCAB * DIM];
    let mut g_ctx: Vec<Vec<f32>> = (0..NUM_CTX).map(|_| vec![0.0f32; VOCAB * DIM]).collect();
    let mut g_proj = vec![0.0f32; HIDDEN * DIM];
    let mut g_head = vec![0.0f32; VOCAB * HIDDEN];

    let (all_hidden, all_ln, all_contexts) = forward_pass(model, tokens, count);
    let total_loss = backward_pass(
        model, &all_hidden, &all_ln, &all_contexts, tokens, count,
        &mut g_embed, &mut g_ctx, &mut g_proj, &mut g_head,
    );

    let n = count as f32;
    for x in g_embed.iter_mut() { *x /= n; }
    for gc in g_ctx.iter_mut() { for x in gc.iter_mut() { *x /= n; } }
    for x in g_proj.iter_mut() { *x /= n; }
    for x in g_head.iter_mut() { *x /= n; }

    let grads = TrainGrads { g_embed, g_ctx, g_proj, g_head };
    (grads, all_hidden, total_loss)
}

/// Forward pass: compute all hidden states and intermediate values.
fn forward_pass(
    model: &NgramModel,
    tokens: &[usize],
    count: usize,
) -> (Vec<Vec<f32>>, Vec<Vec<f32>>, Vec<Vec<usize>>) {
    assert!(count > 0, "forward_pass: count=0");
    let mut all_hidden = Vec::with_capacity(count);
    let mut all_ln = Vec::with_capacity(count);
    let mut all_contexts = Vec::with_capacity(count);

    for i in 0..count {
        let context: Vec<usize> = tokens[i..i + NGRAM].to_vec();
        let t0 = context[NGRAM - 1].min(VOCAB - 1);
        let mut combined = model.embed[t0 * DIM..(t0 + 1) * DIM].to_vec();
        for (ci, cw) in model.ctx_weights.iter().enumerate() {
            let ctx_idx = NGRAM - 2 - ci;
            let t = context[ctx_idx].min(VOCAB - 1);
            let cv = &model.ctx[ci][t * DIM..(t + 1) * DIM];
            for j in 0..DIM {
                combined[j] += cv[j] * cw;
            }
        }
        let ln = model.layer_norm(&combined, 1e-5);
        let mut hidden = vec![0.0f32; HIDDEN];
        for hi in 0..HIDDEN {
            for (j, l) in ln.iter().enumerate() {
                hidden[hi] += model.proj[hi * DIM + j] * l;
            }
            hidden[hi] = hidden[hi].max(0.0);
        }
        all_hidden.push(hidden);
        all_ln.push(ln);
        all_contexts.push(context);
    }

    (all_hidden, all_ln, all_contexts)
}

/// Backward pass: compute gradients from hidden states.
fn backward_pass(
    model: &NgramModel,
    all_hidden: &[Vec<f32>],
    all_ln: &[Vec<f32>],
    all_contexts: &[Vec<usize>],
    tokens: &[usize],
    count: usize,
    g_embed: &mut [f32],
    g_ctx: &mut [Vec<f32>],
    g_proj: &mut [f32],
    g_head: &mut [f32],
) -> f32 {
    assert!(count > 0, "backward_pass: count=0");
    if all_hidden.len() != count {
        eprintln!("WARN: hidden count mismatch: {} != {}, skipping step", all_hidden.len(), count);
        return 0.0;
    }
    assert_eq!(all_ln.len(), count, "ln count mismatch");
    let mut total_loss = 0.0f32;

    for i in 0..count {
        let target = tokens[i + NGRAM].min(VOCAB - 1);
        let hidden = &all_hidden[i];
        let mut d_hidden = vec![0.0f32; HIDDEN];
        let mut logits = model.predict(hidden);
        model.softmax(&mut logits);
        total_loss -= logits[target].max(1e-10).ln();

        for (vi, prob) in logits.iter().enumerate() {
            let grad = prob - if vi == target { 1.0 } else { 0.0 };
            for hi in 0..HIDDEN {
                g_head[vi * HIDDEN + hi] += grad * hidden[hi];
                d_hidden[hi] += grad * model.lm_head[vi * HIDDEN + hi];
            }
        }

        for hi in 0..HIDDEN {
            if all_hidden[i][hi] <= 0.0 {
                continue;
            }
            for di in 0..DIM {
                g_proj[hi * DIM + di] += d_hidden[hi] * all_ln[i][di];
            }
        }

        accumulate_input_grads(
            model, &all_contexts[i], &all_hidden[i], &d_hidden,
            g_embed, g_ctx,
        );
    }

    total_loss / count as f32
}

/// Accumulate gradients for input embeddings.
fn accumulate_input_grads(
    model: &NgramModel,
    context: &[usize],
    hidden: &[f32],
    d_hidden: &[f32],
    g_embed: &mut [f32],
    g_ctx: &mut [Vec<f32>],
) {
    assert!(context.len() >= NGRAM, "context too short");
    let t0 = context[NGRAM - 1].min(VOCAB - 1);
    for di in 0..DIM {
        let mut grad_sum = 0.0f32;
        for hi in 0..HIDDEN {
            if hidden[hi] > 0.0 {
                grad_sum += model.proj[hi * DIM + di] * d_hidden[hi];
            }
        }
        g_embed[t0 * DIM + di] += grad_sum;
        for (ci, cw) in model.ctx_weights.iter().enumerate() {
            let ctx_idx = NGRAM - 2 - ci;
            let t = context[ctx_idx].min(VOCAB - 1);
            g_ctx[ci][t * DIM + di] += cw * grad_sum;
        }
    }
}
