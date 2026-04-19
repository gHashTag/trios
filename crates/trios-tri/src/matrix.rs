//! Ternary matrix operations (∓)
//!
//! Φ3: Dense matrix support for FFN layers.
//!
//! Provides 2D matrix operations for ternary quantized weights,
//! optimized for the FFN gate/up/down layers in the hybrid pipeline.

use super::{Ternary, quantize, compute_scale};

/// A dense ternary matrix in row-major order.
///
/// Used for FFN layer weights and other 2D weight tensors.
/// Stored as a flat vector of ternary values with row-major indexing.
///
/// # Memory
/// - Storage: ~1.58 bits per element (log₂(3))
/// - Compression: 20.25× vs f32, 10.13× vs GF16
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TernaryMatrix {
    /// Flat storage in row-major order
    data: Vec<Ternary>,
    /// Number of rows
    rows: usize,
    /// Number of columns
    cols: usize,
}

impl TernaryMatrix {
    /// Create a ternary matrix from f32 data with optimal scaling.
    ///
    /// # Arguments
    /// * `data` - f32 matrix data in row-major order
    /// * `rows` - Number of rows
    /// * `cols` - Number of columns
    ///
    /// # Panics
    /// Panics if `data.len() != rows * cols`
    ///
    /// # Example
    /// ```
    /// use trios_tri::TernaryMatrix;
    ///
    /// let data = vec![1.0, -0.5, 0.3, -1.5];
    /// let matrix = TernaryMatrix::from_f32(&data, 2, 2);
    /// assert_eq!(matrix.rows(), 2);
    /// assert_eq!(matrix.cols(), 2);
    /// ```
    pub fn from_f32(data: &[f32], rows: usize, cols: usize) -> Self {
        assert_eq!(
            data.len(),
            rows * cols,
            "data length must equal rows * cols"
        );

        let scale = compute_scale(data);
        let ternary_data = quantize(data, scale);

        Self {
            data: ternary_data,
            rows,
            cols,
        }
    }

    /// Create a ternary matrix from a slice of ternary values.
    ///
    /// # Arguments
    /// * `data` - Ternary values in row-major order
    /// * `rows` - Number of rows
    /// * `cols` - Number of columns
    ///
    /// # Panics
    /// Panics if `data.len() != rows * cols`
    pub fn from_ternary(data: Vec<Ternary>, rows: usize, cols: usize) -> Self {
        assert_eq!(
            data.len(),
            rows * cols,
            "data length must equal rows * cols"
        );

        Self {
            data,
            rows,
            cols,
        }
    }

    /// Create a zero matrix.
    ///
    /// All elements are `Ternary::Zero`.
    ///
    /// # Example
    /// ```
    /// use trios_tri::{TernaryMatrix, Ternary};
    ///
    /// let zeros = TernaryMatrix::zeros(3, 4);
    /// assert_eq!(zeros.get(0, 0), Ternary::Zero);
    /// assert_eq!(zeros.get(2, 3), Ternary::Zero);
    /// ```
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            data: vec![Ternary::Zero; rows * cols],
            rows,
            cols,
        }
    }

    /// Create an identity matrix.
    ///
    /// Diagonal elements are `Ternary::PosOne`, off-diagonal are `Ternary::Zero`.
    ///
    /// # Panics
    /// Panics if `rows != cols` (matrix must be square).
    ///
    /// # Example
    /// ```
    /// use trios_tri::{TernaryMatrix, Ternary};
    ///
    /// let id = TernaryMatrix::identity(3);
    /// assert_eq!(id.get(0, 0), Ternary::PosOne);
    /// assert_eq!(id.get(0, 1), Ternary::Zero);
    /// assert_eq!(id.get(2, 2), Ternary::PosOne);
    /// ```
    pub fn identity(n: usize) -> Self {
        assert!(n > 0, "identity matrix size must be positive");

        let mut data = vec![Ternary::Zero; n * n];
        for i in 0..n {
            data[i * n + i] = Ternary::PosOne;
        }

        Self {
            data,
            rows: n,
            cols: n,
        }
    }

    /// Get the number of rows.
    #[inline]
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// Get the number of columns.
    #[inline]
    pub const fn cols(&self) -> usize {
        self.cols
    }

    /// Get the total number of elements.
    #[inline]
    pub const fn len(&self) -> usize {
        self.rows * self.cols
    }

    /// Check if the matrix is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.rows == 0 || self.cols == 0
    }

    /// Get a single element.
    ///
    /// # Arguments
    /// * `row` - Row index (0-based)
    /// * `col` - Column index (0-based)
    ///
    /// # Panics
    /// Panics if `row >= self.rows()` or `col >= self.cols()`
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> Ternary {
        assert!(row < self.rows, "row {} out of bounds (rows = {})", row, self.rows);
        assert!(col < self.cols, "col {} out of bounds (cols = {})", col, self.cols);
        self.data[row * self.cols + col]
    }

    /// Set a single element.
    ///
    /// # Arguments
    /// * `row` - Row index (0-based)
    /// * `col` - Column index (0-based)
    /// * `value` - New ternary value
    ///
    /// # Panics
    /// Panics if `row >= self.rows()` or `col >= self.cols()`
    #[inline]
    pub fn set(&mut self, row: usize, col: usize, value: Ternary) {
        assert!(row < self.rows, "row {} out of bounds (rows = {})", row, self.rows);
        assert!(col < self.cols, "col {} out of bounds (cols = {})", col, self.cols);
        self.data[row * self.cols + col] = value;
    }

    /// Get the raw data slice.
    ///
    /// Returns the internal data in row-major order.
    #[inline]
    pub fn as_slice(&self) -> &[Ternary] {
        &self.data
    }

    /// Get mutable raw data slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [Ternary] {
        &mut self.data
    }

    /// Transpose the matrix.
    ///
    /// Returns a new matrix where rows become columns.
    ///
    /// # Example
    /// ```
    /// use trios_tri::{TernaryMatrix, Ternary};
    ///
    /// let m = TernaryMatrix::from_ternary(
    ///     vec![Ternary::PosOne, Ternary::NegOne,
    ///          Ternary::Zero, Ternary::PosOne],
    ///     2, 2
    /// );
    /// let t = m.transpose();
    /// assert_eq!(t.get(0, 1), Ternary::Zero);
    /// assert_eq!(t.get(1, 0), Ternary::NegOne);
    /// ```
    pub fn transpose(&self) -> Self {
        let mut result = vec![Ternary::Zero; self.rows * self.cols];

        for i in 0..self.rows {
            for j in 0..self.cols {
                result[j * self.rows + i] = self.get(i, j);
            }
        }

        Self {
            data: result,
            rows: self.cols,
            cols: self.rows,
        }
    }

    /// Matrix multiplication.
    ///
    /// Computes `self * other` using ternary arithmetic.
    /// Result clamped to ternary values.
    ///
    /// # Arguments
    /// * `other` - Right-hand side matrix
    ///
    /// # Panics
    /// Panics if `self.cols() != other.rows()`
    ///
    /// # Example
    /// ```
    /// use trios_tri::{TernaryMatrix, Ternary};
    ///
    /// let a = TernaryMatrix::from_ternary(
    ///     vec![Ternary::PosOne, Ternary::NegOne,
    ///          Ternary::Zero, Ternary::PosOne],
    ///     2, 2
    /// );
    /// let result = a.matmul(&a);
    /// assert_eq!(result.rows(), 2);
    /// assert_eq!(result.cols(), 2);
    /// ```
    pub fn matmul(&self, other: &Self) -> Self {
        assert_eq!(
            self.cols,
            other.rows,
            "inner dimensions must match for matmul: {}x{} * {}x{}",
            self.rows,
            self.cols,
            other.rows,
            other.cols
        );

        let rows = self.rows;
        let cols = other.cols;
        let inner = self.cols;

        let mut result = vec![Ternary::Zero; rows * cols];

        for i in 0..rows {
            for j in 0..cols {
                let mut sum: i32 = 0;
                for k in 0..inner {
                    sum += (self.get(i, k) as i8) as i32 * (other.get(k, j) as i8) as i32;
                }

                // Clamp sum to ternary range
                let ternary_val = match sum {
                    2..=i32::MAX => Ternary::PosOne,
                    1 => Ternary::PosOne,
                    0 => Ternary::Zero,
                    -1 => Ternary::NegOne,
                    ..=-2 => Ternary::NegOne,
                };

                result[i * cols + j] = ternary_val;
            }
        }

        Self {
            data: result,
            rows,
            cols,
        }
    }

    /// Element-wise addition with clamping.
    ///
    /// # Panics
    /// Panics if matrices have different dimensions.
    pub fn add(&self, other: &Self) -> Self {
        assert_eq!(
            self.rows,
            other.rows,
            "matrices must have same number of rows"
        );
        assert_eq!(
            self.cols,
            other.cols,
            "matrices must have same number of columns"
        );

        let data = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(&a, &b)| a.add(b))
            .collect();

        Self {
            data,
            rows: self.rows,
            cols: self.cols,
        }
    }

    /// Element-wise subtraction with clamping.
    ///
    /// # Panics
    /// Panics if matrices have different dimensions.
    pub fn sub(&self, other: &Self) -> Self {
        assert_eq!(
            self.rows,
            other.rows,
            "matrices must have same number of rows"
        );
        assert_eq!(
            self.cols,
            other.cols,
            "matrices must have same number of columns"
        );

        let data = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(&a, &b)| a.sub(b))
            .collect();

        Self {
            data,
            rows: self.rows,
            cols: self.cols,
        }
    }

    /// Element-wise multiplication.
    ///
    /// # Panics
    /// Panics if matrices have different dimensions.
    pub fn mul(&self, other: &Self) -> Self {
        assert_eq!(
            self.rows,
            other.rows,
            "matrices must have same number of rows"
        );
        assert_eq!(
            self.cols,
            other.cols,
            "matrices must have same number of columns"
        );

        let data = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(&a, &b)| a.mul(b))
            .collect();

        Self {
            data,
            rows: self.rows,
            cols: self.cols,
        }
    }

    /// Count non-zero elements in the matrix.
    ///
    /// Useful for analyzing sparsity patterns.
    pub fn count_nonzero(&self) -> usize {
        self.data.iter().filter(|&&t| t != Ternary::Zero).count()
    }

    /// Count zero elements in the matrix.
    pub fn count_zero(&self) -> usize {
        self.data.iter().filter(|&&t| t == Ternary::Zero).count()
    }

    /// Calculate sparsity ratio (0.0 = all non-zero, 1.0 = all zero).
    pub fn sparsity(&self) -> f32 {
        self.count_zero() as f32 / self.len() as f32
    }

    /// Convert to f32 matrix with given scale factor.
    ///
    /// # Arguments
    /// * `scale` - Scale factor used during quantization
    pub fn to_f32(&self, scale: f32) -> Vec<f32> {
        self.data.iter().map(|&t| t.to_f32() / scale).collect()
    }

    /// Get the memory footprint in bytes.
    ///
    /// Uses 1.58 bits per element (log₂(3)).
    pub fn memory_bytes(&self) -> usize {
        // 1.58 bits per element = 0.1975 bytes per element
        // Conservative estimate: ceil(1.58 * len / 8)
        (1.58_f32 * self.len() as f32 / 8.0).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_f32() {
        let data = vec![1.0, -0.5, 0.3, -1.5];
        let matrix = TernaryMatrix::from_f32(&data, 2, 2);
        assert_eq!(matrix.rows(), 2);
        assert_eq!(matrix.cols(), 2);
        assert_eq!(matrix.len(), 4);
    }

    #[test]
    fn test_zeros() {
        let zeros = TernaryMatrix::zeros(3, 4);
        assert_eq!(zeros.rows(), 3);
        assert_eq!(zeros.cols(), 4);
        assert_eq!(zeros.get(0, 0), Ternary::Zero);
        assert_eq!(zeros.get(2, 3), Ternary::Zero);
    }

    #[test]
    fn test_identity() {
        let id = TernaryMatrix::identity(3);
        assert_eq!(id.rows(), 3);
        assert_eq!(id.cols(), 3);
        assert_eq!(id.get(0, 0), Ternary::PosOne);
        assert_eq!(id.get(0, 1), Ternary::Zero);
        assert_eq!(id.get(1, 0), Ternary::Zero);
        assert_eq!(id.get(1, 1), Ternary::PosOne);
        assert_eq!(id.get(2, 2), Ternary::PosOne);
    }

    #[test]
    fn test_get_set() {
        let mut m = TernaryMatrix::zeros(2, 2);
        m.set(0, 0, Ternary::PosOne);
        m.set(1, 1, Ternary::NegOne);
        assert_eq!(m.get(0, 0), Ternary::PosOne);
        assert_eq!(m.get(0, 1), Ternary::Zero);
        assert_eq!(m.get(1, 0), Ternary::Zero);
        assert_eq!(m.get(1, 1), Ternary::NegOne);
    }

    #[test]
    fn test_transpose() {
        let m = TernaryMatrix::from_ternary(
            vec![Ternary::PosOne, Ternary::NegOne, Ternary::Zero, Ternary::PosOne],
            2, 2
        );
        let t = m.transpose();
        assert_eq!(t.rows(), 2);
        assert_eq!(t.cols(), 2);
        assert_eq!(t.get(0, 0), Ternary::PosOne);
        assert_eq!(t.get(0, 1), Ternary::Zero);
        assert_eq!(t.get(1, 0), Ternary::NegOne);
        assert_eq!(t.get(1, 1), Ternary::PosOne);
    }

    #[test]
    fn test_matmul() {
        let a = TernaryMatrix::identity(2);
        let b = TernaryMatrix::identity(2);
        let c = a.matmul(&b);
        assert_eq!(c.get(0, 0), Ternary::PosOne);
        assert_eq!(c.get(0, 1), Ternary::Zero);
        assert_eq!(c.get(1, 0), Ternary::Zero);
        assert_eq!(c.get(1, 1), Ternary::PosOne);
    }

    #[test]
    fn test_add() {
        let a = TernaryMatrix::identity(2);
        let b = TernaryMatrix::identity(2);
        let c = a.add(&b);
        // (+1)+(+1) = +1 (clamp), 0+0 = 0
        assert_eq!(c.get(0, 0), Ternary::PosOne);
        assert_eq!(c.get(1, 1), Ternary::PosOne);
    }

    #[test]
    fn test_sub() {
        let a = TernaryMatrix::identity(2);
        let b = TernaryMatrix::identity(2);
        let c = a.sub(&b);
        // All zeros since identity - identity = zero
        assert_eq!(c.get(0, 0), Ternary::Zero);
        assert_eq!(c.get(1, 1), Ternary::Zero);
    }

    #[test]
    fn test_mul() {
        let a = TernaryMatrix::identity(2);
        let b = TernaryMatrix::identity(2);
        let c = a.mul(&b);
        assert_eq!(c.get(0, 0), Ternary::PosOne);
        assert_eq!(c.get(0, 1), Ternary::Zero);
        assert_eq!(c.get(1, 0), Ternary::Zero);
        assert_eq!(c.get(1, 1), Ternary::PosOne);
    }

    #[test]
    fn test_count_nonzero() {
        let m = TernaryMatrix::identity(3);
        assert_eq!(m.count_nonzero(), 3);
        assert_eq!(m.count_zero(), 6);
    }

    #[test]
    fn test_sparsity() {
        let m = TernaryMatrix::identity(4);
        assert!((m.sparsity() - 0.75).abs() < 0.01); // 12/16 = 0.75

        let z = TernaryMatrix::zeros(3, 3);
        assert_eq!(z.sparsity(), 1.0);
    }

    #[test]
    fn test_to_f32() {
        let m = TernaryMatrix::identity(2);
        let f32 = m.to_f32(1.0);
        assert_eq!(f32.len(), 4);
        assert_eq!(f32[0], 1.0); // (0,0)
        assert_eq!(f32[1], 0.0); // (0,1)
        assert_eq!(f32[2], 0.0); // (1,0)
        assert_eq!(f32[3], 1.0); // (1,1)
    }

    #[test]
    fn test_memory_bytes() {
        let m = TernaryMatrix::zeros(100, 100);
        let bytes = m.memory_bytes();
        // 10000 * 1.58 bits / 8 ≈ 1975 bytes
        assert!(bytes > 1900 && bytes < 2100);
    }

    #[test]
    #[should_panic(expected = "row")]
    fn test_get_out_of_bounds_row() {
        let m = TernaryMatrix::zeros(2, 2);
        m.get(3, 0);
    }

    #[test]
    #[should_panic(expected = "col")]
    fn test_get_out_of_bounds_col() {
        let m = TernaryMatrix::zeros(2, 2);
        m.get(0, 3);
    }

    #[test]
    #[should_panic(expected = "inner dimensions")]
    fn test_matmul_wrong_dims() {
        let a = TernaryMatrix::zeros(2, 3);
        let b = TernaryMatrix::zeros(4, 2);
        a.matmul(&b);
    }
}
