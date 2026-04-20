//! Ternary arithmetic operations (∓)
//!
//! Φ3: Basic operations for {-1, 0, +1} values.
//!
//! Implements arithmetic with clamping to the ternary set.

use super::Ternary;

impl Ternary {
    pub fn from_f32_batch(values: &[f32]) -> Vec<Self> {
        values.iter().map(|&v| Self::from_f32(v)).collect()
    }

    pub fn clamp_add(self, other: Self) -> Self {
        let sum = self as i8 + other as i8;
        match sum {
            2 | 1 => Ternary::PosOne,
            0 => Ternary::Zero,
            -1 | -2 => Ternary::NegOne,
            _ => Ternary::Zero,
        }
    }

    pub fn clamp_sub(self, other: Self) -> Self {
        let diff = self as i8 - other as i8;
        match diff {
            2 | 1 => Ternary::PosOne,
            0 => Ternary::Zero,
            -1 | -2 => Ternary::NegOne,
            _ => Ternary::Zero,
        }
    }
}

impl std::ops::Add for Ternary {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.clamp_add(rhs)
    }
}

impl std::ops::Sub for Ternary {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.clamp_sub(rhs)
    }
}

impl std::ops::Mul for Ternary {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let product = (self as i8) * (rhs as i8);
        match product {
            1 => Ternary::PosOne,
            0 => Ternary::Zero,
            -1 => Ternary::NegOne,
            _ => Ternary::Zero,
        }
    }
}

impl std::ops::Neg for Ternary {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Ternary::PosOne => Ternary::NegOne,
            Ternary::NegOne => Ternary::PosOne,
            Ternary::Zero => Ternary::Zero,
        }
    }
}

/// Compute the dot product of two ternary vectors.
///
/// Returns the sum of element-wise products as an i32.
/// For fully ternary vectors, this is equivalent to counting
/// matching signs minus opposing signs.
///
/// # Arguments
/// * `a` - First ternary vector
/// * `b` - Second ternary vector (must be same length)
///
/// # Returns
/// Dot product as i32
///
/// # Panics
/// Panics if vectors have different lengths.
///
/// # Example
/// ```
/// use trios_tri::{Ternary, dot_product};
///
/// let a = vec![Ternary::PosOne, Ternary::Zero, Ternary::NegOne];
/// let b = vec![Ternary::PosOne, Ternary::Zero, Ternary::PosOne];
/// assert_eq!(dot_product(&a, &b), 0); // (+1*+1) + (0*0) + (-1*+1) = 1 + 0 - 1 = 0
/// ```
pub fn dot_product(a: &[Ternary], b: &[Ternary]) -> i32 {
    assert_eq!(a.len(), b.len(), "vectors must have same length");

    a.iter()
        .zip(b.iter())
        .map(|(ta, tb)| (*ta as i8) * (*tb as i8))
        .map(|p| p as i32)
        .sum()
}

/// Compute the L1 distance (Manhattan distance) between two ternary vectors.
///
/// # Example
/// ```
/// use trios_tri::{Ternary, l1_distance};
///
/// let a = vec![Ternary::PosOne, Ternary::Zero, Ternary::NegOne];
/// let b = vec![Ternary::PosOne, Ternary::PosOne, Ternary::NegOne];
/// assert_eq!(l1_distance(&a, &b), 1); // only one element differs
/// ```
pub fn l1_distance(a: &[Ternary], b: &[Ternary]) -> i32 {
    assert_eq!(a.len(), b.len(), "vectors must have same length");

    a.iter()
        .zip(b.iter())
        .map(|(ta, tb)| (*ta as i8 - *tb as i8).abs())
        .map(|d| d as i32)
        .sum()
}

/// Count the number of non-zero elements in a ternary vector.
///
/// This is useful for measuring the effective sparsity of ternarized weights.
///
/// # Example
/// ```
/// use trios_tri::{Ternary, vec_count_nonzero as count_nonzero};
///
/// let v = vec![Ternary::PosOne, Ternary::Zero, Ternary::NegOne, Ternary::Zero];
/// assert_eq!(count_nonzero(&v), 2);
/// ```
pub fn count_nonzero(vec: &[Ternary]) -> usize {
    vec.iter().filter(|&&t| t != Ternary::Zero).count()
}

/// Count the number of zero elements in a ternary vector.
///
/// # Example
/// ```
/// use trios_tri::{Ternary, vec_count_zero as count_zero};
///
/// let v = vec![Ternary::PosOne, Ternary::Zero, Ternary::NegOne, Ternary::Zero];
/// assert_eq!(count_zero(&v), 2);
/// ```
pub fn count_zero(vec: &[Ternary]) -> usize {
    vec.iter().filter(|&&t| t == Ternary::Zero).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(Ternary::PosOne.clamp_add(Ternary::PosOne), Ternary::PosOne);
        assert_eq!(Ternary::PosOne.clamp_add(Ternary::Zero), Ternary::PosOne);
        assert_eq!(Ternary::PosOne.clamp_add(Ternary::NegOne), Ternary::Zero);
        assert_eq!(Ternary::Zero.clamp_add(Ternary::Zero), Ternary::Zero);
        assert_eq!(Ternary::NegOne.clamp_add(Ternary::NegOne), Ternary::NegOne);
    }

    #[test]
    fn test_sub() {
        assert_eq!(Ternary::PosOne.clamp_sub(Ternary::PosOne), Ternary::Zero);
        assert_eq!(Ternary::PosOne.clamp_sub(Ternary::Zero), Ternary::PosOne);
        assert_eq!(Ternary::PosOne.clamp_sub(Ternary::NegOne), Ternary::PosOne);
        assert_eq!(Ternary::NegOne.clamp_sub(Ternary::PosOne), Ternary::NegOne);
    }

    #[test]
    fn test_mul() {
        assert_eq!(Ternary::PosOne * Ternary::PosOne, Ternary::PosOne);
        assert_eq!(Ternary::PosOne * Ternary::Zero, Ternary::Zero);
        assert_eq!(Ternary::PosOne * Ternary::NegOne, Ternary::NegOne);
        assert_eq!(Ternary::NegOne * Ternary::NegOne, Ternary::PosOne);
    }

    #[test]
    fn test_neg() {
        assert_eq!(-Ternary::PosOne, Ternary::NegOne);
        assert_eq!(-Ternary::NegOne, Ternary::PosOne);
        assert_eq!(-Ternary::Zero, Ternary::Zero);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![Ternary::PosOne, Ternary::Zero, Ternary::NegOne];
        let b = vec![Ternary::PosOne, Ternary::Zero, Ternary::PosOne];
        assert_eq!(dot_product(&a, &b), 0);

        let c = vec![Ternary::PosOne, Ternary::PosOne, Ternary::PosOne];
        let d = vec![Ternary::PosOne, Ternary::PosOne, Ternary::PosOne];
        assert_eq!(dot_product(&c, &d), 3);
    }

    #[test]
    fn test_l1_distance() {
        let a = vec![Ternary::PosOne, Ternary::Zero, Ternary::NegOne];
        let b = vec![Ternary::PosOne, Ternary::PosOne, Ternary::NegOne];
        assert_eq!(l1_distance(&a, &b), 1);
    }

    #[test]
    fn test_count_nonzero() {
        let v = vec![
            Ternary::PosOne,
            Ternary::Zero,
            Ternary::NegOne,
            Ternary::Zero,
        ];
        assert_eq!(count_nonzero(&v), 2);
    }

    #[test]
    fn test_count_zero() {
        let v = vec![
            Ternary::PosOne,
            Ternary::Zero,
            Ternary::NegOne,
            Ternary::Zero,
        ];
        assert_eq!(count_zero(&v), 2);
    }

    #[test]
    fn test_ops_traits() {
        let a = Ternary::PosOne;
        let b = Ternary::NegOne;

        assert_eq!(a + b, Ternary::Zero);
        assert_eq!(a - b, Ternary::PosOne);
        assert_eq!(a * b, Ternary::NegOne);
        assert_eq!(-a, Ternary::NegOne);
    }

    #[test]
    fn test_from_f32_batch() {
        let values = vec![1.0, -1.0, 0.0, 0.6, -0.6];
        let ternary = Ternary::from_f32_batch(&values);
        assert_eq!(ternary.len(), 5);
        assert_eq!(ternary[0], Ternary::PosOne);
        assert_eq!(ternary[1], Ternary::NegOne);
        assert_eq!(ternary[2], Ternary::Zero);
        assert_eq!(ternary[3], Ternary::PosOne);
        assert_eq!(ternary[4], Ternary::NegOne);
    }
}
