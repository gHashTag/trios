//! trios-data: FineWeb data loader for Trinity training (Φ5.5)
//!
//! Provides batched data loading for the FineWeb dataset.
//! Currently a stub implementation that returns dummy tokens.

/// Batched FineWeb data loader.
///
/// In production, this would load from actual FineWeb dataset files.
/// For now, it's a stub that returns dummy token IDs for testing.
pub struct FineWebBatch {
    batch_size: usize,
    offset: usize,
    /// Maximum dummy samples to generate (end of epoch)
    max_samples: usize,
}

impl FineWebBatch {
    /// Create a new FineWeb batch loader.
    ///
    /// # Arguments
    ///
    /// * `data_path` - Path to FineWeb dataset (currently unused in stub)
    /// * `batch_size` - Number of sequences per batch
    pub fn new(_data_path: &str, batch_size: usize) -> Self {
        Self {
            batch_size,
            offset: 0,
            max_samples: 1000, // Stub: limit to 1000 dummy samples
        }
    }

    /// Get the next batch of token sequences.
    ///
    /// Returns `None` when the dataset is exhausted.
    ///
    /// # Returns
    ///
    /// * `Some(Vec<Vec<u32>>)` - Batch of token sequences (each sequence is a vector of token IDs)
    /// * `None` - End of dataset
    pub fn next_batch(&mut self) -> Option<Vec<Vec<u32>>> {
        if self.offset >= self.max_samples {
            return None;
        }

        // Stub: returns dummy token sequences
        // Each sequence has 128 tokens, all with token ID 42
        let batch = (0..self.batch_size)
            .map(|_| vec![42u32; 128])
            .collect();

        self.offset += self.batch_size;
        Some(batch)
    }

    /// Get the current offset in the dataset.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Get the batch size.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Reset the loader to the beginning of the dataset.
    pub fn reset(&mut self) {
        self.offset = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_batch_loader() {
        let loader = FineWebBatch::new("/dummy/path", 32);
        assert_eq!(loader.batch_size(), 32);
        assert_eq!(loader.offset(), 0);
    }

    #[test]
    fn test_next_batch_returns_correct_size() {
        let mut loader = FineWebBatch::new("/dummy/path", 4);
        let batch = loader.next_batch();

        assert!(batch.is_some());
        let batch = batch.unwrap();
        assert_eq!(batch.len(), 4);

        // Each sequence should have 128 tokens
        for seq in &batch {
            assert_eq!(seq.len(), 128);
        }
    }

    #[test]
    fn test_multiple_batches() {
        let mut loader = FineWebBatch::new("/dummy/path", 100);

        // First batch
        let batch1 = loader.next_batch();
        assert!(batch1.is_some());
        assert_eq!(loader.offset(), 100);

        // Second batch
        let batch2 = loader.next_batch();
        assert!(batch2.is_some());
        assert_eq!(loader.offset(), 200);

        // ... continue until exhausted
        while loader.next_batch().is_some() {}

        // Should be exhausted after 1000 samples / 100 batch_size = 10 batches
        assert_eq!(loader.offset(), 1000);
        assert!(loader.next_batch().is_none());
    }

    #[test]
    fn test_reset() {
        let mut loader = FineWebBatch::new("/dummy/path", 50);

        // Consume some batches
        loader.next_batch();
        loader.next_batch();
        assert_eq!(loader.offset(), 100);

        // Reset
        loader.reset();
        assert_eq!(loader.offset(), 0);

        // Should be able to get batches again
        assert!(loader.next_batch().is_some());
    }

    #[test]
    fn test_dummy_tokens() {
        let mut loader = FineWebBatch::new("/dummy/path", 2);
        let batch = loader.next_batch().unwrap();

        // Verify dummy token IDs are 42
        assert_eq!(batch[0], vec![42u32; 128]);
        assert_eq!(batch[1], vec![42u32; 128]);
    }
}
