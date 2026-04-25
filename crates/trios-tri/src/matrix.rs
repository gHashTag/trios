//! 2D matrix operations for ternary values

use crate::Ternary;

/// Ternary matrix for FFN layer operations
#[derive(Debug, Clone)]
pub struct TernaryMatrix {
    data: Vec<Ternary>,
    rows: usize,
    cols: usize,
}

impl TernaryMatrix {
    /// Create a new ternary matrix from f32 data
    pub fn from_f32(data: &[f32], rows: usize, cols: usize) -> Self {
        assert_eq!(data.len(), rows * cols, "data size must match rows * cols");
        Self {
            data: data.iter().map(|&x| Ternary::from_f32(x)).collect(),
            rows,
            cols,
        }
    }

    /// Get number of rows
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get number of columns
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get a reference to the underlying data
    pub fn data(&self) -> &[Ternary] {
        &self.data
    }

    /// Matrix multiplication with another ternary matrix
    ///
    /// Returns the result as i32 values (since ternary dot products are integers)
    pub fn matmul(&self, other: &TernaryMatrix) -> Vec<i32> {
        assert_eq!(
            self.cols, other.rows,
            "matrix dimensions incompatible for multiplication"
        );

        let mut result = vec![0i32; self.rows * other.cols];

        for i in 0..self.rows {
            for j in 0..other.cols {
                let mut sum = 0i32;
                for k in 0..self.cols {
                    let a = self.data[i * self.cols + k];
                    let b = other.data[k * other.cols + j];
                    sum += (a.as_i8() as i32) * (b.as_i8() as i32);
                }
                result[i * other.cols + j] = sum;
            }
        }

        result
    }

    /// Transpose the matrix
    pub fn transpose(&self) -> Self {
        let mut data = vec![Ternary::Zero; self.rows * self.cols];
        for i in 0..self.rows {
            for j in 0..self.cols {
                data[j * self.rows + i] = self.data[i * self.cols + j];
            }
        }
        Self {
            data,
            rows: self.cols,
            cols: self.rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ternary_matrix_creation() {
        let data = vec![1.0, -0.8, 0.3, 1.5];
        let matrix = TernaryMatrix::from_f32(&data, 2, 2);
        assert_eq!(matrix.rows(), 2);
        assert_eq!(matrix.cols(), 2);
    }

    #[test]
    fn test_ternary_matrix_transpose() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let matrix = TernaryMatrix::from_f32(&data, 2, 2);
        let transposed = matrix.transpose();
        assert_eq!(transposed.rows(), 2);
        assert_eq!(transposed.cols(), 2);
    }

    #[test]
    fn test_ternary_matrix_matmul() {
        let a_data = vec![1.0, 0.0, -1.0, 1.0];
        let b_data = vec![1.0, 1.0, 0.0, -1.0];
        let a = TernaryMatrix::from_f32(&a_data, 2, 2);
        let b = TernaryMatrix::from_f32(&b_data, 2, 2);
        let result = a.matmul(&b);
        assert_eq!(result.len(), 4);
    }
}
