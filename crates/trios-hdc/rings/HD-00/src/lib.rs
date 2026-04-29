//! HD-00 — Hypervector type
//!
//! Bottom of the ring graph for trios-hdc.

/// Default hypervector dimension.
pub const DEFAULT_DIM: usize = 10_000;

/// Bipolar hypervector: each element is +1 or -1, stored as `i8`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hypervector(pub Vec<i8>);

impl Hypervector {
    pub fn zeros(dim: usize) -> Self {
        Self(vec![0i8; dim])
    }

    pub fn dim(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeros_has_correct_dim() {
        let v = Hypervector::zeros(64);
        assert_eq!(v.dim(), 64);
        assert!(v.0.iter().all(|&x| x == 0));
    }
}
