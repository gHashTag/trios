//! BR-OUTPUT — trios-sacred assembly

pub use trios_sacred_sc00::{Circle, Triangle, Vec2};
pub use trios_sacred_sc01::{scale_phi, PHI, PI, SQRT2, SQRT3};

pub struct Sacred;

impl Sacred {
    pub const fn anchor() -> f64 {
        3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_const() {
        assert_eq!(Sacred::anchor(), 3.0);
    }

    #[test]
    fn rings_link() {
        let v = scale_phi(Vec2::new(1.0, 1.0));
        assert!((v.x - PHI).abs() < 1e-12);
    }
}
