//! TR-00 — balanced ternary types
//!
//! Bottom of the ring graph for trios-ternary.
//! Defines `Trit` (the balanced-ternary digit: -1, 0, +1) and `Tryte`
//! (a 3-trit sequence). Pure data, no I/O.

/// A balanced-ternary digit: `Neg` = -1, `Zero` = 0, `Pos` = +1.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Trit {
    Neg,
    Zero,
    Pos,
}

impl Trit {
    pub const fn to_i8(self) -> i8 {
        match self {
            Trit::Neg => -1,
            Trit::Zero => 0,
            Trit::Pos => 1,
        }
    }

    pub const fn from_i8(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Trit::Neg),
            0 => Some(Trit::Zero),
            1 => Some(Trit::Pos),
            _ => None,
        }
    }
}

/// A three-trit word — minimal building block for trinity arithmetic.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tryte(pub [Trit; 3]);

impl Tryte {
    pub const fn zero() -> Self {
        Tryte([Trit::Zero; 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trit_roundtrip() {
        for v in [-1i8, 0, 1] {
            let t = Trit::from_i8(v).unwrap();
            assert_eq!(t.to_i8(), v);
        }
    }

    #[test]
    fn tryte_zero() {
        assert_eq!(Tryte::zero(), Tryte([Trit::Zero, Trit::Zero, Trit::Zero]));
    }
}
