// trios-ca-mask: Fibonacci attention mask for efficient sparse attention (Φ5.4)
// Creates causal mask with Fibonacci pattern for sparse attention

/// Creates a Fibonacci-patterned causal attention mask.
///
/// # Arguments
/// * `seq_len` - Sequence length
/// * `max_offset` - Maximum Fibonacci offset to allow
///
/// # Returns
/// A flattened `seq_len x seq_len` boolean mask where:
/// - `true` = position is attended
/// - `false` = position is masked
///
/// # Pattern
/// Future positions are only attended if their offset is a Fibonacci number
/// and within `max_offset`. This creates sparse attention following Fibonacci
/// spacing for efficiency.
pub fn fibonacci_ca_mask(seq_len: usize, max_offset: usize) -> Vec<bool> {
    let mut mask = vec![true; seq_len * seq_len];
    for i in 0..seq_len {
        for j in 0..seq_len {
            if j > i {
                // Future positions: check Fibonacci offset
                let offset = j - i;
                mask[i * seq_len + j] = is_fibonacci(offset) && offset <= max_offset;
            }
        }
    }
    mask
}

/// Checks if a number is a Fibonacci number using the mathematical property:
/// A number n is Fibonacci if and only if 5*n^2 + 4 or 5*n^2 - 4 is a perfect square.
fn is_fibonacci(n: usize) -> bool {
    if n == 0 {
        return true;
    }
    let test1 = 5.0 * (n as f32).powi(2) + 4.0;
    let test2 = 5.0 * (n as f32).powi(2) - 4.0;
    is_perfect_square(test1) || is_perfect_square(test2)
}

/// Checks if a float is a perfect square.
fn is_perfect_square(x: f32) -> bool {
    if x < 0.0 {
        return false;
    }
    let sqrt = x.sqrt();
    let rounded = sqrt.round() as i32;
    (rounded as f32).powi(2) == x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_fibonacci() {
        assert!(is_fibonacci(0));
        assert!(is_fibonacci(1));
        assert!(is_fibonacci(2));
        assert!(is_fibonacci(3));
        assert!(is_fibonacci(5));
        assert!(is_fibonacci(8));
        assert!(is_fibonacci(13));
        assert!(is_fibonacci(21));
        assert!(is_fibonacci(34));
        assert!(!is_fibonacci(4));
        assert!(!is_fibonacci(6));
        assert!(!is_fibonacci(7));
        assert!(!is_fibonacci(10));
        assert!(!is_fibonacci(15));
    }

    #[test]
    fn test_fibonacci_ca_mask_causal() {
        let seq_len = 5;
        let mask = fibonacci_ca_mask(seq_len, 100);
        for i in 0..seq_len {
            for j in 0..seq_len {
                // Past and present should always be true
                if j <= i {
                    assert!(mask[i * seq_len + j], "i={}, j={} should be true", i, j);
                }
            }
        }
    }

    #[test]
    fn test_fibonacci_ca_mask_pattern() {
        let seq_len = 10;
        let max_offset = 8;
        let mask = fibonacci_ca_mask(seq_len, max_offset);
        // Check specific Fibonacci offsets are allowed
        // From position 0, Fibonacci offsets 1, 2, 3, 5, 8 should be true
        assert!(mask[0 * seq_len + 1], "offset 1 should be true");
        assert!(mask[0 * seq_len + 2], "offset 2 should be true");
        assert!(mask[0 * seq_len + 3], "offset 3 should be true");
        assert!(!mask[0 * seq_len + 4], "offset 4 should be false");
        assert!(mask[0 * seq_len + 5], "offset 5 should be true");
        assert!(!mask[0 * seq_len + 6], "offset 6 should be false");
        assert!(!mask[0 * seq_len + 7], "offset 7 should be false");
        assert!(mask[0 * seq_len + 8], "offset 8 should be true");
        // Offset 9 exceeds max_offset
        assert!(!mask[0 * seq_len + 9], "offset 9 should be false (exceeds max_offset)");
    }

    #[test]
    fn test_fibonacci_ca_mask_size() {
        let seq_len = 7;
        let mask = fibonacci_ca_mask(seq_len, 10);
        assert_eq!(mask.len(), seq_len * seq_len);
    }

    #[test]
    fn test_fibonacci_ca_mask_empty() {
        let mask = fibonacci_ca_mask(0, 10);
        assert!(mask.is_empty());
    }

    #[test]
    fn test_fibonacci_ca_mask_single() {
        let mask = fibonacci_ca_mask(1, 10);
        assert_eq!(mask.len(), 1);
        assert!(mask[0], "single position should be true");
    }

    #[test]
    fn test_is_perfect_square() {
        assert!(is_perfect_square(0.0));
        assert!(is_perfect_square(1.0));
        assert!(is_perfect_square(4.0));
        assert!(is_perfect_square(9.0));
        assert!(is_perfect_square(16.0));
        assert!(!is_perfect_square(2.0));
        assert!(!is_perfect_square(3.0));
        assert!(!is_perfect_square(5.0));
        assert!(!is_perfect_square(-1.0));
    }
}
