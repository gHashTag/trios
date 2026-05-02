//! φ-band learning rate sampler.
//!
//! Samples learning rates within INV-1 safe bounds [0.00382, 0.00618].

use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Learning rate sampler with φ-band constraints.
pub struct LrSampler {
    rng: StdRng,
}

/// Error that can occur during sampling.
#[derive(Debug, thiserror::Error)]
pub enum LrSampleError {
    #[error("LR outside φ-band: {0}")]
    OutOfBand(f64),
}

impl LrSampler {
    /// Create a new sampler with random seed.
    pub fn new() -> Self {
        // Use rand::rng() to get entropy, then seed StdRng
        let mut rng = rand::rng();
        let seed: [u8; 32] = rng.random();

        LrSampler {
            rng: StdRng::from_seed(seed),
        }
    }

    /// Create a new sampler with fixed seed (for reproducibility).
    pub fn with_seed(seed: u64) -> Self {
        LrSampler {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Sample a learning rate within INV-1 φ-band.
    ///
    /// Returns LR in [INV1_LR_SAFE_LO, INV1_LR_SAFE_HI].
    pub fn sample(&mut self) -> f64 {
        use trios_algorithm_arena::invariants::INV1_LR_SAFE_LO;
        use trios_algorithm_arena::invariants::INV1_LR_SAFE_HI;

        self.rng.random_range(INV1_LR_SAFE_LO..=INV1_LR_SAFE_HI)
    }

    /// Sample using φ-anchored champion LR.
    ///
    /// Returns LR in [CHAMPION_LR/φ, CHAMPION_LR*φ].
    pub fn sample_champion_band(&mut self) -> f64 {
        use trios_algorithm_arena::invariants::INV1_CHAMPION_LR;

        let phi = 1.618_033_988_749_895;
        let lo = INV1_CHAMPION_LR / phi;
        let hi = INV1_CHAMPION_LR * phi;

        self.rng.random_range(lo..=hi)
    }

    /// Sample near a given LR with Gaussian noise.
    pub fn sample_near(&mut self, center: f64, std_dev: f64) -> f64 {
        use rand_distr::{Distribution, Normal};

        let normal = Normal::new(center, std_dev).unwrap();
        normal.sample(&mut self.rng)
    }
}

impl Default for LrSampler {
    fn default() -> Self {
        Self::new()
    }
}
