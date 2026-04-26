//! FineWeb data loader for IGLA training
//!
//! Loads FineWeb dataset in binary format (uint16 tokens).
//! Format: 256 x 4-byte header + token data

use anyhow::{anyhow, Result};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// FineWeb dataset header constants
const MAGIC_NUMBER: u32 = 20240520;
const HEADER_SIZE: usize = 1024; // 256 x 4-byte integers

/// FineWeb binary dataset loader
pub struct FineWebDataset {
    pub tokens: Vec<u16>,
    pub vocab_size: usize,
}

impl FineWebDataset {
    /// Load FineWeb data from binary file
    ///
    /// # Format
    /// - 1024-byte header (256 x 4-byte integers):
    ///   - bytes 0-4: magic number (20240520)
    ///   - bytes 4-8: version (1)
    ///   - bytes 8-12: number of tokens
    /// - Token data: uint16 big-endian
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut file = File::open(path)
            .map_err(|e| anyhow!("Failed to open {}: {}", path.display(), e))?;

        // Read header
        let mut header_bytes = [0u8; HEADER_SIZE];
        file.read_exact(&mut header_bytes)
            .map_err(|e| anyhow!("Failed to read header from {}: {}", path.display(), e))?;

        // Parse header
        let magic = u32::from_le_bytes(header_bytes[0..4].try_into().unwrap());
        let version = u32::from_le_bytes(header_bytes[4..8].try_into().unwrap());
        let num_tokens = u32::from_le_bytes(header_bytes[8..12].try_into().unwrap()) as usize;

        if magic != MAGIC_NUMBER {
            return Err(anyhow!("Invalid magic number: {} (expected {})", magic, MAGIC_NUMBER));
        }
        if version != 1 {
            return Err(anyhow!("Unsupported version: {} (expected 1)", version));
        }

        // Read token data (uint16)
        let mut token_bytes = vec![0u8; num_tokens * 2];
        file.read_exact(&mut token_bytes)
            .map_err(|e| anyhow!("Failed to read tokens from {}: {}", path.display(), e))?;

        // Convert to u16 tokens (little-endian)
        let tokens = token_bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        Ok(Self {
            tokens,
            vocab_size: 50257, // GPT-2 vocab size
        })
    }

    /// Create a fallback dataset with synthetic data if file not found
    pub fn fallback() -> Self {
        // "The quick brown fox jumps over the lazy dog" repeated
        let base = b"The quick brown fox jumps over the lazy dog. ";
        let repeated = base.repeat(100);
        let tokens: Vec<u16> = repeated.iter().map(|&b| b as u16).collect();

        Self {
            tokens,
            vocab_size: 256,
        }
    }

    /// Get the number of tokens in the dataset
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Check if dataset is empty
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Get a slice of tokens
    pub fn get_slice(&self, start: usize, end: usize) -> &[u16] {
        &self.tokens[start..end.min(self.tokens.len())]
    }

    /// Sample a random sequence for training
    pub fn sample_sequence(&self, seq_len: usize, rng_state: &mut u64) -> Vec<u32> {
        if self.tokens.len() <= seq_len + 1 {
            return self.tokens.iter().map(|&t| t as u32).collect();
        }

        *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let offset = (*rng_state as usize) % (self.tokens.len() - seq_len - 1);

        self.tokens[offset..offset + seq_len + 1]
            .iter()
            .map(|&t| t as u32)
            .collect()
    }

    /// Get a contiguous slice for evaluation
    pub fn get_eval_batch(&self, max_tokens: usize) -> Vec<u32> {
        let n = max_tokens.min(self.tokens.len());
        self.tokens[..n].iter().map(|&t| t as u32).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_dataset() {
        let dataset = FineWebDataset::fallback();
        assert!(!dataset.is_empty());
        assert!(dataset.len() > 0);
    }

    #[test]
    fn test_sample_sequence() {
        let dataset = FineWebDataset::fallback();
        let mut rng_state = 42u64;
        let seq = dataset.sample_sequence(10, &mut rng_state);
        assert_eq!(seq.len(), 11); // seq_len + 1 for target
    }

    #[test]
    fn test_get_eval_batch() {
        let dataset = FineWebDataset::fallback();
        let batch = dataset.get_eval_batch(100);
        assert!(batch.len() <= 100);
    }
}
