//! trios-brain — Memory module for TRIOS
//!
//! R0: Pure Rust stub implementation (HashMap-based)
//! R1: FFI envelope (planned)
//! R2: Persistence / HSLM hook (planned)

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Brain error types
#[derive(Debug, Clone, PartialEq)]
pub enum BrainError {
    KeyNotFound,
    InvalidKey,
    StorageError,
    SerializationError,
}

impl std::fmt::Display for BrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrainError::KeyNotFound => write!(f, "Key not found in brain"),
            BrainError::InvalidKey => write!(f, "Invalid key format"),
            BrainError::StorageError => write!(f, "Storage operation failed"),
            BrainError::SerializationError => write!(f, "Serialization failed"),
        }
    }
}

impl std::error::Error for BrainError {}

/// Thread-local brain storage (R0 stub)
struct BrainStorage {
    data: HashMap<String, Vec<u8>>,
}

impl BrainStorage {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    fn remember(&mut self, key: &str, value: &[u8]) {
        self.data.insert(key.to_string(), value.to_vec());
    }

    fn recall(&self, key: &str) -> Option<Vec<u8>> {
        self.data.get(key).cloned()
    }

    fn forget(&mut self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }

    fn count(&self) -> usize {
        self.data.len()
    }
}

/// Global brain instance (thread-safe for R0)
static BRAIN: LazyLock<Mutex<BrainStorage>> = LazyLock::new(|| Mutex::new(BrainStorage::new()));

/// Store a value in brain memory
///
/// # Arguments
/// * `key` - Memory key (UTF-8 string, non-empty)
/// * `value` - Binary value to store
///
/// # Returns
/// * `Ok(())` - Value stored successfully
/// * `Err(BrainError::InvalidKey)` - Key is empty
///
/// # Example
/// ```no_run
/// use trios_brain::brain_remember;
///
/// brain_remember("test_key", b"hello world").unwrap();
/// ```
pub fn brain_remember(key: &str, value: &[u8]) -> Result<(), BrainError> {
    if key.is_empty() {
        return Err(BrainError::InvalidKey);
    }

    let mut brain = BRAIN.lock().map_err(|_| BrainError::StorageError)?;

    brain.remember(key, value);
    Ok(())
}

/// Recall a value from brain memory
///
/// # Arguments
/// * `key` - Memory key to retrieve
///
/// # Returns
/// * `Ok(Vec<u8>)` - Found value
/// * `Err(BrainError::KeyNotFound)` - Key doesn't exist
/// * `Err(BrainError::InvalidKey)` - Key is empty
///
/// # Example
/// ```no_run
/// use trios_brain::{brain_remember, brain_recall};
///
/// brain_remember("test_key", b"hello world").unwrap();
/// let value = brain_recall("test_key").unwrap();
/// assert_eq!(value, b"hello world");
/// ```
pub fn brain_recall(key: &str) -> Result<Vec<u8>, BrainError> {
    if key.is_empty() {
        return Err(BrainError::InvalidKey);
    }

    let brain = BRAIN.lock().map_err(|_| BrainError::StorageError)?;

    brain.recall(key).ok_or(BrainError::KeyNotFound)
}

/// Forget a value from brain memory
///
/// # Arguments
/// * `key` - Memory key to remove
///
/// # Returns
/// * `Ok(true)` - Key was found and removed
/// * `Ok(false)` - Key didn't exist
/// * `Err(BrainError::InvalidKey)` - Key is empty
pub fn brain_forget(key: &str) -> Result<bool, BrainError> {
    if key.is_empty() {
        return Err(BrainError::InvalidKey);
    }

    let mut brain = BRAIN.lock().map_err(|_| BrainError::StorageError)?;

    Ok(brain.forget(key))
}

/// Get total number of stored memories
pub fn brain_count() -> usize {
    BRAIN.lock().map(|brain| brain.count()).unwrap_or(0)
}

/// Clear all memories (for testing only)
#[cfg(test)]
pub fn brain_clear() {
    let mut brain = BRAIN.lock().unwrap();
    *brain = BrainStorage::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remember_and_recall() {
        let key = "test_recall_7d2e";
        brain_forget(key).ok();
        let value = b"hello world";

        brain_remember(key, value).unwrap();
        let recalled = brain_recall(key).unwrap();

        assert_eq!(recalled, value);
    }

    #[test]
    fn test_overwrite() {
        let key = "test_overwrite_3b1f";
        brain_forget(key).ok();

        brain_remember(key, b"original").unwrap();
        brain_remember(key, b"updated").unwrap();

        let recalled = brain_recall(key).unwrap();
        assert_eq!(recalled, b"updated");
    }

    #[test]
    fn test_missing_key() {
        let result = brain_recall("nonexistent_key_9a4c");
        assert_eq!(result, Err(BrainError::KeyNotFound));
    }

    #[test]
    fn test_invalid_key() {
        let empty_result = brain_remember("", b"value");
        assert_eq!(empty_result, Err(BrainError::InvalidKey));

        let recall_empty = brain_recall("");
        assert_eq!(recall_empty, Err(BrainError::InvalidKey));
    }

    #[test]
    fn test_forget() {
        let key = "test_forget_5f8a";
        brain_forget(key).ok();
        brain_remember(key, b"value").unwrap();

        let removed = brain_forget(key).unwrap();
        assert!(removed);

        let recall_result = brain_recall(key);
        assert_eq!(recall_result, Err(BrainError::KeyNotFound));
    }

    #[test]
    fn test_forget_nonexistent() {
        let removed = brain_forget("nonexistent_2e7d").unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_count() {
        let k1 = "test_count_a1b2";
        let k2 = "test_count_c3d4";
        brain_forget(k1).ok();
        brain_forget(k2).ok();

        let before = brain_count();

        brain_remember(k1, b"value1").unwrap();
        brain_remember(k2, b"value2").unwrap();

        assert_eq!(brain_count(), before + 2);

        brain_forget(k1).unwrap();

        assert_eq!(brain_count(), before + 1);

        brain_forget(k2).ok();
    }

    #[test]
    fn test_binary_data() {
        let key = "test_binary_8f3a";
        brain_forget(key).ok();
        let binary_data: Vec<u8> = (0..256).map(|i| i as u8).collect();

        brain_remember(key, &binary_data).unwrap();
        let recalled = brain_recall(key).unwrap();

        assert_eq!(recalled, binary_data);
        assert_eq!(recalled.len(), 256);
        brain_forget(key).ok();
    }
}
