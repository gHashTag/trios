//! BR-OUTPUT — trios-physics assembly

pub use trios_physics_ph00::{ALPHA, C, G, H, HBAR};
pub use trios_physics_ph01::{de_broglie_wavelength, rest_energy};

pub struct Physics;

impl Physics {
    pub const fn anchor() -> f64 {
        3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_link() {
        let e = rest_energy(1.0);
        assert!(e > 0.0);
        assert_eq!(Physics::anchor(), 3.0);
    }
}
