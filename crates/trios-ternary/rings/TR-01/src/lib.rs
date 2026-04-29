//! TR-01 — balanced ternary arithmetic
//!
//! Operations on Trits and Trytes. Depends only on TR-00.

use trios_ternary_tr00::Trit;

/// Negate a Trit (involution).
pub const fn neg(t: Trit) -> Trit {
    match t {
        Trit::Neg => Trit::Pos,
        Trit::Zero => Trit::Zero,
        Trit::Pos => Trit::Neg,
    }
}

/// Add two trits as i8, then map back if in range.
pub fn add_saturating(a: Trit, b: Trit) -> i8 {
    a.to_i8() + b.to_i8()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neg_involution() {
        for t in [Trit::Neg, Trit::Zero, Trit::Pos] {
            assert_eq!(neg(neg(t)), t);
        }
    }

    #[test]
    fn add_pos_neg_zero() {
        assert_eq!(add_saturating(Trit::Pos, Trit::Neg), 0);
        assert_eq!(add_saturating(Trit::Pos, Trit::Pos), 2);
    }
}
