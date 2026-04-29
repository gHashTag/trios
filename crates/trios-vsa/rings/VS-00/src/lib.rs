//! VS-00 — Symbol type and VSA algebra
//!
//! Bottom of the ring graph for trios-vsa.

/// A bipolar symbol vector — a sequence of +1/-1 values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol(pub Vec<i8>);

impl Symbol {
    pub fn new(dim: usize) -> Self {
        Self(vec![0i8; dim])
    }

    pub fn dim(&self) -> usize {
        self.0.len()
    }

    /// Inverse for bipolar symbols equals the symbol itself (involution).
    pub fn inverse(&self) -> Self {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_dim() {
        let s = Symbol::new(8);
        assert_eq!(s.dim(), 8);
    }

    #[test]
    fn inverse_involution() {
        let s = Symbol(vec![1, -1, 1, -1]);
        assert_eq!(s.inverse(), s);
    }
}
