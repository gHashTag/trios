//! Arithmetic operations for ternary values

use crate::Ternary;

/// Dot product of two ternary vectors
pub fn dot_product(a: &[Ternary], b: &[Ternary]) -> i32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x.as_i8() as i32) * (y.as_i8() as i32))
        .sum()
}

/// L1 distance between two ternary vectors
pub fn l1_distance(a: &[Ternary], b: &[Ternary]) -> i32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x.as_i8() - y.as_i8()).abs() as i32)
        .sum()
}

/// Count non-zero elements in a vector
pub fn count_nonzero(v: &[Ternary]) -> usize {
    v.iter().filter(|&&t| t != Ternary::Zero).count()
}

/// Count zero elements in a vector
pub fn count_zero(v: &[Ternary]) -> usize {
    v.iter().filter(|&&t| t == Ternary::Zero).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_product() {
        let a = vec![Ternary::PosOne, Ternary::Zero, Ternary::NegOne];
        let b = vec![Ternary::PosOne, Ternary::PosOne, Ternary::NegOne];
        assert_eq!(dot_product(&a, &b), 2); // 1*1 + 0*1 + (-1)*(-1) = 2
    }

    #[test]
    fn test_l1_distance() {
        let a = vec![Ternary::PosOne, Ternary::Zero];
        let b = vec![Ternary::NegOne, Ternary::PosOne];
        assert_eq!(l1_distance(&a, &b), 3); // |1-(-1)| + |0-1| = 2 + 1 = 3
    }

    #[test]
    fn test_count_nonzero() {
        let v = vec![Ternary::PosOne, Ternary::Zero, Ternary::NegOne];
        assert_eq!(count_nonzero(&v), 2);
    }

    #[test]
    fn test_count_zero() {
        let v = vec![Ternary::PosOne, Ternary::Zero, Ternary::NegOne, Ternary::Zero];
        assert_eq!(count_zero(&v), 2);
    }
}
