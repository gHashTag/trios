//! BR-OUTPUT — trios-hdc assembly

pub use trios_hdc_hd00::{Hypervector, DEFAULT_DIM};
pub use trios_hdc_hd01::{bind, bundle, similarity};

pub struct Hdc;

impl Hdc {
    pub const fn anchor() -> f64 {
        3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_link() {
        let a = Hypervector(vec![1, -1, 1]);
        let r = bind(&a, &a);
        assert!(similarity(&a, &a) > 0.99);
        assert_eq!(r.dim(), 3);
    }
}
