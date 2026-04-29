//! BR-OUTPUT — trios-ternary assembly
//!
//! Re-exports public surface of TR-00 and TR-01.

pub use trios_ternary_tr00::{Trit, Tryte};
pub use trios_ternary_tr01::{add_saturating, neg};

/// Trinity witness: -1 + 0 + 1 = 0.
pub const TRINITY_SUM: i8 = -1 + 0 + 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trinity_sum_zero() {
        assert_eq!(TRINITY_SUM, 0);
    }

    #[test]
    fn rings_link() {
        let z = Tryte::zero();
        assert_eq!(neg(Trit::Pos), Trit::Neg);
        assert_eq!(z, Tryte([Trit::Zero; 3]));
    }
}
