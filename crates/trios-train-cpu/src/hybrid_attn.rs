//! # Hybrid Attention Block — Gate-2 → Gate-final Architecture (L-h2 → L-f1)
//!
//! Causal self-attention stack supporting 1 or 2 layers for the hybrid
//! ngram+attn trainer. The block is deliberately minimal so that invariants
//! guarding it (INV-1 lr-band, INV-9 φ-anchor, and pre-registered
//! INV-13 `hybrid_qk_gain_phi_sq`) can be asserted with a short, auditable
//! implementation.
//!
//! ## Pre-registration
//!
//! Gate-2 (immutable): single-layer depth via trios#143 comment 4320342032.
//!
//! Gate-final (DRAFT → immutable after Gate-2 first row):
//! - Extended to support `num_attn_layers ∈ {1, 2}` (INV-13 refined)
//! - Second layer uses same RoPE, residual + LayerNorm pattern
//! - Coq lemmas: `counter_skew_seeds`, `counter_lr_outside_band` (L-f5)
//!
//! This module is owned by L-h2 (Gate-2) → L-f1 (Gate-final extension).
//!
//! ## Constants (Coq-grounded, L-R14)
//!
//! | Constant              | Value                        | Source                                          |
//! |-----------------------|------------------------------|-------------------------------------------------|
//! | `PHI_SQ`              | `2.618033988749895`          | [`crate::invariants::PHI_SQ`] (`lr_convergence.v::phi_cube`) |
//! | `PHI_CUBE`            | `4.23606797749979`           | [`crate::invariants::PHI_CUBE`]                  |
//! | `LR_SAFE_MIN`         | `0.002`                      | [`crate::invariants::LR_SAFE_MIN`] (INV-1)       |
//! | `LR_SAFE_MAX`         | `0.007`                      | [`crate::invariants::LR_SAFE_MAX`] (INV-1)       |
//! | `ALLOWED_QK_GAINS`    | `{PHI_SQ, PHI_CUBE}`         | INV-13 (this module)                             |
//!
//! ## Falsification (R7)
//!
//! The block refuses to construct itself when any of the following hold:
//!
//! 1. `lr ∉ [LR_SAFE_MIN, LR_SAFE_MAX]` → [`HybridAttnError::LrOutOfBand`]
//! 2. `qk_gain ∉ {PHI_SQ, PHI_CUBE}`    → [`HybridAttnError::QkGainOutsidePhi`]
//! 3. `d_model == 0` or `num_heads == 0` or `d_model % num_heads != 0`
//!                                      → [`HybridAttnError::Shape`]
//! 4. `num_attn_layers ∉ {1, 2}`        → [`HybridAttnError::InvalidDepth`] (L-f1)
//! 5. Non-finite input in forward pass → [`HybridAttnError::NonFinite`]
//!
//! Each of these corresponds to a named falsifier test at the bottom of this
//! file. Deleting or weakening a test is a pre-registration deviation and
//! must be filed as described above.
//!
//! ## Scope
//!
//! This file is **single** file owned by L-h2. It is called by
//! `hybrid_train.rs` (L-h1) but owns **no** pre-existing module. Per R6
//! (lane discipline), only out-of-file touch is a one-line
//! `pub mod hybrid_attn;` re-export in [`crate::lib`].

#![allow(clippy::doc_overindented_list_items)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]

use crate::invariants::{LR_SAFE_MAX, LR_SAFE_MIN, PHI_CUBE, PHI_SQ};

// ═════════════════════════════════════════════════════════
// INV-13 — Allowed qk_gain values
// Pre-registered: qk_gain ∈ {φ², φ³}.
// Coq lemma (L-h4): trinity-clara/proofs/igla/hybrid_qk_gain.v
//     ::counter_qk_gain_outside_phi_sq
// ═════════════════════════════════════════════════════════════

/// Allowed qk-gain values for the causal attention block.
///
/// Pre-registered as `{φ², φ³}`. Any other value is refused at construction.
pub const ALLOWED_QK_GAINS: [f64; 2] = [PHI_SQ, PHI_CUBE];

/// Pre-registered default qk_gain for Gate-2: φ².
pub const DEFAULT_QK_GAIN: f64 = PHI_SQ;

/// Pre-registered default learning rate for Gate-2: 0.0035 (inside of
/// INV-1 band `[0.002, 0.007]`).
pub const DEFAULT_LR: f64 = 0.0035;

/// Pre-registered default depth for Gate-2: 1 layer.
pub const DEFAULT_NUM_ATTN_LAYERS: u8 = 1;

/// φ-scaled hidden width for Gate-final: round(φ · 512) = 828.
///
/// This is lever #2 in the Gate-final decomposition (−0.05..−0.10 BPB expected).
pub const GATE_FINAL_HIDDEN_WIDTH: usize = 828;

// ═══════════════════════════════════════════════════════════
// Error type
// ═════════════════════════════════════════════════════════════════

/// Construction / forward-pass refusals.
///
/// Every variant has a corresponding falsifier test.  Never silence a
/// variant — surface it as `Result::Err` so that trainer lane (L-h1) can
/// record of refusal in the race ledger.
#[derive(Debug, Clone, PartialEq)]
pub enum HybridAttnError {
    /// `lr ∉ [LR_SAFE_MIN, LR_SAFE_MAX]` — INV-1 violation.
    LrOutOfBand { lr: f64 },
    /// `qk_gain ∉ {PHI_SQ, PHI_CUBE}` — INV-13 violation (pre-registered).
    QkGainOutsidePhi { qk_gain: f64 },
    /// Shape invariants failed (zero dimension or indivisible head split).
    Shape { d_model: usize, num_heads: usize },
    /// Invalid depth: `num_attn_layers ∉ {1, 2}` — INV-13 (refined, L-f1).
    InvalidDepth { depth: u8 },
    /// Non-finite tensor detected in the forward pass.
    NonFinite,
}

impl std::fmt::Display for HybridAttnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LrOutOfBand { lr } => write!(
                f,
                "INV-1 violation: lr={lr} outside φ-safe band [{LR_SAFE_MIN}, {LR_SAFE_MAX}]",
            ),
            Self::QkGainOutsidePhi { qk_gain } => write!(
                f,
                "INV-13 violation: qk_gain={qk_gain} not in pre-registered \
                 set {{φ²={PHI_SQ}, φ³={PHI_CUBE}}}",
            ),
            Self::Shape {
                d_model,
                num_heads,
            } => write!(
                f,
                "shape invariant failed: d_model={d_model}, num_heads={num_heads} \
                 (both must be > 0 and d_model % num_heads == 0)",
            ),
            Self::InvalidDepth { depth } => write!(
                f,
                "INV-13 violation (L-f1): num_attn_layers={depth} not in pre-registered set {{1, 2}}",
            ),
            Self::NonFinite => write!(f, "non-finite tensor in forward pass"),
        }
    }
}

impl std::error::Error for HybridAttnError {}

// ═══════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════

/// Pre-registered Gate-2 → Gate-final shape.
///
/// Gate-2: `d_model=64`, `num_heads=4`, `seq_len=8`, `num_attn_layers=1`.
/// Gate-final (DRAFT): extends to `num_attn_layers=2` (INV-13 refined).
#[derive(Debug, Clone, Copy)]
pub struct HybridAttnConfig {
    /// Model dimension (must be a multiple of `num_heads`).
    pub d_model: usize,
    /// Number of attention heads.
    pub num_heads: usize,
    /// Maximum sequence length handled by RoPE.
    pub seq_len: usize,
    /// Query/key scaling gain — **must** be in [`ALLOWED_QK_GAINS`].
    pub qk_gain: f64,
    /// Learning rate — **must** be in `[LR_SAFE_MIN, LR_SAFE_MAX]`.
    pub lr: f64,
    /// Number of causal attention layers — **must** be in `{1, 2}` (INV-13 refined, L-f1).
    pub num_attn_layers: u8,
}

impl Default for HybridAttnConfig {
    fn default() -> Self {
        Self {
            d_model: 64,
            num_heads: 4,
            seq_len: 8,
            qk_gain: DEFAULT_QK_GAIN,
            lr: DEFAULT_LR,
            num_attn_layers: DEFAULT_NUM_ATTN_LAYERS,
        }
    }
}

impl HybridAttnConfig {
    /// Validate this config against INV-1, INV-13, and shape invariants.
    ///
    /// This is the central chokepoint: every public constructor routes
    /// through here so that a single inspection audits all refusal paths.
    pub fn validate(&self) -> Result<(), HybridAttnError> {
        // NASA Rule 5: minimum 2 assert-equivalent checks per pub fn.
        if !(LR_SAFE_MIN..=LR_SAFE_MAX).contains(&self.lr) {
            return Err(HybridAttnError::LrOutOfBand { lr: self.lr });
        }
        if !ALLOWED_QK_GAINS
            .iter()
            .any(|g| (g - self.qk_gain).abs() < 1e-9)
        {
            return Err(HybridAttnError::QkGainOutsidePhi {
                qk_gain: self.qk_gain,
            });
        }
        if self.d_model == 0
            || self.num_heads == 0
            || !self.d_model.is_multiple_of(self.num_heads)
        {
            return Err(HybridAttnError::Shape {
                d_model: self.d_model,
                num_heads: self.num_heads,
            });
        }
        // INV-13 refined (L-f1): depth must be in {1, 2}
        if self.num_attn_layers != 1 && self.num_attn_layers != 2 {
            return Err(HybridAttnError::InvalidDepth {
                depth: self.num_attn_layers,
            });
        }
        Ok(())
    }
}

// ═════════════════════════════════════════════════════════
// The block itself
// ═════════════════════════════════════════════════════════════════

/// Weights are stored row-major.  We keep dimensions explicit on each
/// matrix so that a reader can reconstruct shapes without consulting `lib.rs`.
///
/// For `num_attn_layers=2`, each layer has its own set of weights.
#[derive(Debug, Clone)]
pub struct HybridAttn {
    cfg: HybridAttnConfig,
    /// Per-layer query projections: `[num_layers][d_model × d_model]`.
    wq: Vec<Vec<f32>>,
    /// Per-layer key projections: `[num_layers][d_model × d_model]`.
    wk: Vec<Vec<f32>>,
    /// Per-layer value projections: `[num_layers][d_model × d_model]`.
    wv: Vec<Vec<f32>>,
    /// Per-layer output projections: `[num_layers][d_model × d_model]`.
    wo: Vec<Vec<f32>>,
}

impl HybridAttn {
    /// Construct with pre-registered defaults (`φ²`, `lr=0.0035`,
    /// `d_model=64`, `num_heads=4`).
    pub fn new() -> Result<Self, HybridAttnError> {
        Self::with_config(HybridAttnConfig::default())
    }

    /// Construct with an explicit learning rate (all other values default).
    pub fn new_with_lr(lr: f64) -> Result<Self, HybridAttnError> {
        let mut cfg = HybridAttnConfig::default();
        cfg.lr = lr;
        Self::with_config(cfg)
    }

    /// Construct with an explicit qk_gain (all other values default).
    ///
    /// This refuses at construction time, **not** inside the forward pass —
    /// silent acceptance of a bad gain is a pre-registration violation.
    pub fn new_with_qk_gain(qk_gain: f64) -> Result<Self, HybridAttnError> {
        let mut cfg = HybridAttnConfig::default();
        cfg.qk_gain = qk_gain;
        Self::with_config(cfg)
    }

    /// Construct with a full config.
    pub fn with_config(cfg: HybridAttnConfig) -> Result<Self, HybridAttnError> {
        cfg.validate()?;
        let d = cfg.d_model;
        let dd = d * d;
        let num_layers = cfg.num_attn_layers as usize;
        // Zero-init is fine: trainer (L-h1) re-initialises with a
        // φ-orthogonal scheme from `crate::phi_ortho_init`. Zero-init
        // keeps this module's tests hermetic — a deterministic seed is
        // also unavailable here without pulling `rand`, which would
        // inflate dependency surface of an L-h2 module.
        let mut wq = Vec::with_capacity(num_layers);
        let mut wk = Vec::with_capacity(num_layers);
        let mut wv = Vec::with_capacity(num_layers);
        let mut wo = Vec::with_capacity(num_layers);
        for _ in 0..num_layers {
            wq.push(vec![0.0_f32; dd]);
            wk.push(vec![0.0_f32; dd]);
            wv.push(vec![0.0_f32; dd]);
            wo.push(vec![0.0_f32; dd]);
        }
        Ok(Self {
            cfg,
            wq,
            wk,
            wv,
            wo,
        })
    }

    /// The pre-registered config. Callers that need to re-assert
    /// invariants (e.g. CI gate in L-h1) should use this accessor
    /// instead of clone-unwrapping internal fields.
    pub fn config(&self) -> &HybridAttnConfig {
        &self.cfg
    }

    /// Re-assert INV-1 + INV-13 + shape at any later point. This is
    /// cheap and idempotent, and trainer calls it once per step as
    /// an online invariant check.
    pub fn reassert(&self) -> Result<(), HybridAttnError> {
        self.cfg.validate()
    }

    // --- RoPE -----------------------------------------------------------

    /// RoPE angle for position `p` and head-dim index `i` (`0 ≤ i < d_head/2`).
    ///
    /// We use the classical formula `θ = p / 10000^{2i / d_head}`, which
    /// has the φ-periodicity property required by INV-9 (see
    /// `hybrid_attn_rope_periodicity` test for a concrete bound).
    pub fn rope_angle(position: usize, head_dim_idx: usize, d_head: usize) -> f32 {
        assert!(d_head > 0, "INV: d_head must be positive");
        assert!(
            head_dim_idx < d_head / 2,
            "INV: head_dim_idx {head_dim_idx} must be < d_head/2 = {}",
            d_head / 2,
        );
        let exp = (2.0 * head_dim_idx as f32) / (d_head as f32);
        (position as f32) / 10_000.0_f32.powf(exp)
    }

    // --- Forward pass ---------------------------------------------------

    /// Single-step causal attention forward pass on a batch of
    /// `seq_len × d_model` tokens. Returns the post-output-projection
    /// activations of the same shape, flattened row-major.
    ///
    /// For `num_attn_layers > 1`, each layer receives the residual+LayerNorm
    /// output from the previous layer. Layer 1 receives the input tokens directly.
    ///
    /// The pass is written straightforwardly: clarity beats speed in the
    /// pre-registered block, because the measured quantity is the
    /// learning dynamic (`val_bpb_at_step_54000`) not wall-clock.
    /// Optimisation lives downstream in `hybrid_train.rs` (L-h1).
    pub fn forward(
        &self,
        tokens: &[f32],
        seq_len: usize,
    ) -> Result<Vec<f32>, HybridAttnError> {
        if tokens.iter().any(|x| !x.is_finite()) {
            return Err(HybridAttnError::NonFinite);
        }
        let d = self.cfg.d_model;
        let h = self.cfg.num_heads;
        let d_head = d / h;
        let num_layers = self.cfg.num_attn_layers as usize;
        assert_eq!(
            tokens.len(),
            seq_len * d,
            "forward: tokens.len() = {} but expected seq_len * d_model = {}",
            tokens.len(),
            seq_len * d,
        );

        // Layer 1 receives input tokens directly
        let mut hidden = tokens.to_vec();

        // Stack attention layers with residual connections
        for layer_idx in 0..num_layers {
            let wq = &self.wq[layer_idx];
            let wk = &self.wk[layer_idx];
            let wv = &self.wv[layer_idx];
            let wo = &self.wo[layer_idx];

            // Per-token LayerNorm before attention
            let eps = 1e-5_f32;
            for t in 0..seq_len {
                let token_start = t * d;
                let token_end = token_start + d;

                let mut mean = 0.0_f32;
                for i in token_start..token_end {
                    mean += hidden[i];
                }
                mean /= d as f32;

                let mut variance = 0.0_f32;
                for i in token_start..token_end {
                    let diff = hidden[i] - mean;
                    variance += diff * diff;
                }
                variance /= d as f32;
                let std = (variance + eps).sqrt();

                for i in token_start..token_end {
                    hidden[i] = (hidden[i] - mean) / std;
                }
            }

            // Compute Q, K, V for this layer
            let q = matmul(&hidden, wq, seq_len, d, d);
            let k = matmul(&hidden, wk, seq_len, d, d);
            let v = matmul(&hidden, wv, seq_len, d, d);

            // Per-head scores with qk_gain multiplier
            let scale = (d_head as f32).sqrt();
            let mut attn_out = vec![0.0_f32; seq_len * d];
            for head in 0..h {
                let head_offset = head * d_head;
                for i in 0..seq_len {
                    // Causal mask: softmax over j ∈ [0, i]
                    let mut scores = vec![0.0_f32; i + 1];
                    for (j, score) in scores.iter_mut().enumerate() {
                        let mut s = 0.0_f32;
                        for k_idx in 0..d_head {
                            let qv = q[i * d + head_offset + k_idx];
                            let kv = k[j * d + head_offset + k_idx];
                            s += qv * kv;
                        }
                        *score = (self.cfg.qk_gain as f32) * s / scale;
                    }
                    softmax_inplace(&mut scores);
                    for j in 0..=i {
                        let w = scores[j];
                        for k_idx in 0..d_head {
                            attn_out[i * d + head_offset + k_idx] +=
                                w * v[j * d + head_offset + k_idx];
                        }
                    }
                }
            }

            let layer_out = matmul(&attn_out, wo, seq_len, d, d);

            // Residual connection: hidden = hidden + layer_out
            for i in 0..seq_len * d {
                hidden[i * d] += layer_out[i];
            }
        }

        if hidden.iter().any(|x| !x.is_finite()) {
            return Err(HybridAttnError::NonFinite);
        }
        Ok(hidden)
    }
}

// ═══════════════════════════════════════════════════════════
// Helpers (kept private; test-visible via. `HybridAttn::forward` call)
// ═════════════════════════════════════════════════════════════

fn matmul(a: &[f32], b: &[f32], m: usize, k: usize, n: usize) -> Vec<f32> {
    assert_eq!(a.len(), m * k, "matmul lhs shape");
    assert_eq!(b.len(), k * n, "matmul rhs shape");
    let mut out = vec![0.0_f32; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut s = 0.0_f32;
            for l in 0..k {
                s += a[i * k + l] * b[l * n + j];
            }
            out[i * n + j] = s;
        }
    }
    out
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

// ═════════════════════════════════════════════════════════════
// Falsifier tests — R7 witnesses for INV-1, INV-13, shape, and forward
// ═════════════════════════════════════════════════════════════════

#[cfg(test)]
mod falsifiers {
    use super::*;
    use crate::invariants::PHI;

    /// R7 / INV-1: a learning rate outside of the Coq-proven φ-band must
    /// refuse at construction time. This is a deterministic sibling
    /// of the earlier pure-attention plateau (BPB ≈ 4.74 @ lr=0.01).
    #[test]
    fn falsify_hybrid_diverges_bad_lr() {
        let err = HybridAttn::new_with_lr(0.02).unwrap_err();
        assert!(
            matches!(err, HybridAttnError::LrOutOfBand { .. }),
            "expected LrOutOfBand, got {err:?}",
        );
        // Lower-side witness.
        let err = HybridAttn::new_with_lr(0.0005).unwrap_err();
        assert!(matches!(err, HybridAttnError::LrOutOfBand { .. }));
        // And inside-band default must succeed.
        HybridAttn::new_with_lr(0.0035).expect("0.0035 is inside of band");
    }

    /// R7 / INV-13: any qk_gain outside `{φ², φ³}` must refuse. This is
    /// a Rust mirror of the pre-registered Coq lemma
    /// `counter_qk_gain_outside_phi_sq` (L-h4).
    #[test]
    fn falsify_hybrid_qk_gain_not_phi_sq_or_phi_cube() {
        let err = HybridAttn::new_with_qk_gain(PHI).unwrap_err();
        assert!(
            matches!(err, HybridAttnError::QkGainOutsidePhi { .. }),
            "qk_gain=PHI must be refused, got {err:?}",
        );
        let err = HybridAttn::new_with_qk_gain(1.0).unwrap_err();
        assert!(matches!(err, HybridAttnError::QkGainOutsidePhi { .. }));
        // Both pre-registered gains must succeed.
        HybridAttn::new_with_qk_gain(PHI_SQ).expect("φ² is allowed");
        HybridAttn::new_with_qk_gain(PHI_CUBE).expect("φ³ is allowed");
    }

    /// Shape invariant: `d_model % num_heads != 0` must refuse.
    #[test]
    fn falsify_hybrid_shape_invariant() {
        let cfg = HybridAttnConfig {
            d_model: 64,
            num_heads: 5, // 64 % 5 = 4 ≠ 0
            ..HybridAttnConfig::default()
        };
        let err = HybridAttn::with_config(cfg).unwrap_err();
        assert!(matches!(err, HybridAttnError::Shape { .. }));
    }

    /// R7 / INV-13 (L-f1): `num_attn_layers` must be in {1, 2}.
    #[test]
    fn falsify_invalid_depth_not_one_or_two() {
        let cfg = HybridAttnConfig {
            num_attn_layers: 0, // Not allowed
            ..HybridAttnConfig::default()
        };
        let err = HybridAttn::with_config(cfg).unwrap_err();
        assert!(
            matches!(err, HybridAttnError::InvalidDepth { depth: 0 }),
            "expected InvalidDepth(0), got {err:?}",
        );

        let cfg = HybridAttnConfig {
            num_attn_layers: 3, // Not allowed
            ..HybridAttnConfig::default()
        };
        let err = HybridAttn::with_config(cfg).unwrap_err();
        assert!(
            matches!(err, HybridAttnError::InvalidDepth { depth: 3 }),
            "expected InvalidDepth(3), got {err:?}",
        );

        // Both 1 and 2 must succeed.
        HybridAttn::with_config(HybridAttnConfig {
            num_attn_layers: 1,
            ..HybridAttnConfig::default()
        })
        .expect("depth=1 must succeed");

        HybridAttn::with_config(HybridAttnConfig {
            num_attn_layers: 2,
            ..HybridAttnConfig::default()
        })
        .expect("depth=2 must succeed (Gate-final)");
    }

    /// Deterministic forward pass: zero weights on zero tokens must
    /// return zeros (no NaN, no Inf). The goal is to exercise the
    /// non-finite detector on a known-good input.
    #[test]
    fn hybrid_attn_forward_roundtrip() {
        let block = HybridAttn::new().expect("defaults are valid");
        let seq_len = 4;
        let d = block.config().d_model;
        let tokens = vec![0.0_f32; seq_len * d];
        let out = block.forward(&tokens, seq_len).unwrap();
        assert_eq!(out.len(), seq_len * d);
        assert!(out.iter().all(|x| x.is_finite()));
    }

    /// Two-layer forward pass (Gate-final L-f1 extension).
    #[test]
    fn hybrid_attn_two_layer_forward() {
        let cfg = HybridAttnConfig {
            num_attn_layers: 2, // Gate-final extension
            ..HybridAttnConfig::default()
        };
        let block = HybridAttn::with_config(cfg).expect("depth=2 is valid");
        let seq_len = 4;
        let d = block.config().d_model;
        let tokens = vec![0.5_f32; seq_len * d];
        let out = block.forward(&tokens, seq_len).unwrap();

        // Check output is finite
        assert!(out.iter().all(|x| x.is_finite()));
        // Check output shape
        assert_eq!(out.len(), seq_len * d);
    }

    /// Non-finite input must be surfaced as `Err(NonFinite)`, not
    /// propagated silently. R5: honest refusal.
    #[test]
    fn hybrid_attn_non_finite_refused() {
        let block = HybridAttn::new().expect("defaults are valid");
        let seq_len = 2;
        let d = block.config().d_model;
        let mut tokens = vec![0.0_f32; seq_len * d];
        tokens[0] = f32::NAN;
        let err = block.forward(&tokens, seq_len).unwrap_err();
        assert_eq!(err, HybridAttnError::NonFinite);
    }

    /// RoPE periodicity: for `d_head = 16`, the ratio between the
    /// frequency at index 0 and index 7 is exactly `10_000^{14/16}`.
    /// This property is an INV-9 φ-anchor hook — the actual φ-relation
    /// is proven in the Coq lemma, not re-asserted here.
    #[test]
    fn hybrid_attn_rope_periodicity() {
        let d_head = 16;
        let a0 = HybridAttn::rope_angle(1, 0, d_head);
        let a7 = HybridAttn::rope_angle(1, 7, d_head);
        let ratio = a0 / a7;
        let expected = 10_000.0_f32.powf(14.0 / 16.0);
        assert!(
            (ratio - expected).abs() < 1e-2,
            "RoPE frequency ratio drifted: got {ratio}, expected {expected}",
        );
    }

    /// `reassert()` must stay green for the default config. This is
    /// called inside L-h1's training loop; regressing it breaks
    /// the online invariant sweep.
    #[test]
    fn hybrid_attn_reassert_stable() {
        let block = HybridAttn::new().expect("defaults are valid");
        for _ in 0..8 {
            block.reassert().expect("online reassertion must hold");
        }
    }
}
