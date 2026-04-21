#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ternary {
    MinusOne,
    Zero,
    PosOne,
}

impl Ternary {
    pub fn from_i8(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Ternary::MinusOne),
            0 => Some(Ternary::Zero),
            1 => Some(Ternary::PosOne),
            _ => None,
        }
    }

    pub fn to_i8(self) -> i8 {
        match self {
            Ternary::MinusOne => -1,
            Ternary::Zero => 0,
            Ternary::PosOne => 1,
        }
    }

    pub fn to_f64(self) -> f64 {
        self.to_i8() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for v in [-1_i8, 0, 1] {
            assert_eq!(Ternary::from_i8(v).unwrap().to_i8(), v);
        }
        assert!(Ternary::from_i8(2).is_none());
    }
}
