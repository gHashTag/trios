//! IGLA-GF16 Training Pipeline for Parameter Golf

pub mod model;
pub mod phi_schedule;
pub mod trinity_init;
pub mod ca_mask;
pub mod data;
pub mod train;
pub mod eval;

#[derive(Debug, Clone)]
pub struct TrainingConfig {
    pub iterations: usize,
    pub lr_schedule: LRSchedule,
    pub use_phi_physics: bool,
    pub batch_tokens: usize,
    pub val_every: usize,
    pub output_dir: String,
}

#[derive(Debug, Clone, Copy)]
pub enum LRSchedule {
    Phi,
    Cosine,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            iterations: 20000,
            lr_schedule: LRSchedule::Phi,
            use_phi_physics: true,
            batch_tokens: 524288,
            val_every: 34,
            output_dir: "outputs/igla_gf16".to_string(),
        }
    }
}

pub fn estimate_model_size(vocab: usize, d_model: usize, n_layers: usize) -> f64 {
    let embedding = (vocab * d_model * 2) as f64 / (1024.0 * 1024.0);
    let attention = (n_layers * 4 * d_model * d_model * 2) as f64 / (1024.0 * 1024.0);
    let ffn = (n_layers * 3 * d_model * 232 * 2) as f64 / (1024.0 * 1024.0);
    embedding + attention + ffn
}
