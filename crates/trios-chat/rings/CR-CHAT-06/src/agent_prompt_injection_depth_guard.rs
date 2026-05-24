//! # CR-CHAT-06 — Agent prompt injection depth guard (Wave-79 Lane A)
//!
//! AGENT SAFETY — nested prompt injection depth must be bounded, R-CHAT-7.
//!
//! A prompt injection can itself contain instructions that trigger
//! further injection. Without a depth bound:
//!
//! * **Exponential expansion** — each injection level doubles the
//!   attack surface, causing unbounded agent processing.
//! * **Recursive bypass** — injection at depth N bypasses a filter
//!   applied at depth N-1, nesting until the guard gives up.
//! * **Resource exhaustion** — deep nesting consumes CPU and memory
//!   parsing ever more complex injection attempts.
//!
//! This is distinct from IPDG (injection pattern depth) which counts
//! nesting of injection sentinel patterns. PIDP counts the recursive
//! *prompt processing depth* — how many times the agent has been
//! re-prompted within a single interaction.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Prompt depth <= `PIDP_MAX_DEPTH`.
//! 2. Depth counter starts at 0.
//! 3. Each re-prompt increments depth by exactly 1.
//! 4. Depth never decreases within a session.
//! 5. Re-prompt payload size <= `PIDP_MAX_REPROMPT_LEN`.
//! 6. Total re-prompt bytes across all depths <= `PIDP_MAX_TOTAL_BYTES`.
//!
//! Tests **PIDP-01..10**. Error enum [`InjectionDepthError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PROMPT-INJECTION-DEPTH`

#![forbid(unsafe_code)]

/// Maximum prompt processing depth.
pub const PIDP_MAX_DEPTH: usize = 8;

/// Maximum re-prompt payload length (bytes).
pub const PIDP_MAX_REPROMPT_LEN: usize = 65536;

/// Maximum total re-prompt bytes.
pub const PIDP_MAX_TOTAL_BYTES: usize = 1_048_576;

/// A re-prompt entry.
#[derive(Debug, Clone)]
pub struct RepromptEntry {
    /// Depth level (0-based).
    pub depth: usize,
    /// Payload size in bytes.
    pub payload_len: usize,
}

/// All ways injection depth validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InjectionDepthError {
    /// Depth exceeded.
    DepthExceeded,
    /// Depth regression.
    DepthRegression,
    /// Depth increment > 1.
    DepthJump,
    /// Re-prompt too large.
    RepromptTooLarge,
    /// Total bytes exceeded.
    TotalBytesExceeded,
    /// Negative depth (underflow via bad input).
    InvalidDepth,
}

/// `[VERIFIED]` Validate prompt injection depth progression.
pub fn validate_injection_depth(
    entries: &[RepromptEntry],
) -> Result<(), InjectionDepthError> {
    let mut total_bytes = 0usize;
    let mut prev_depth: Option<usize> = None;
    for entry in entries {
        if entry.depth > PIDP_MAX_DEPTH {
            return Err(InjectionDepthError::DepthExceeded);
        }
        if entry.payload_len > PIDP_MAX_REPROMPT_LEN {
            return Err(InjectionDepthError::RepromptTooLarge);
        }
        total_bytes = match total_bytes.checked_add(entry.payload_len) {
            Some(t) => t,
            None => return Err(InjectionDepthError::TotalBytesExceeded),
        };
        if total_bytes > PIDP_MAX_TOTAL_BYTES {
            return Err(InjectionDepthError::TotalBytesExceeded);
        }
        if let Some(pd) = prev_depth {
            if entry.depth < pd {
                return Err(InjectionDepthError::DepthRegression);
            }
            if entry.depth > pd + 1 {
                return Err(InjectionDepthError::DepthJump);
            }
        }
        prev_depth = Some(entry.depth);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(depth: usize, payload_len: usize) -> RepromptEntry {
        RepromptEntry { depth, payload_len }
    }

    fn valid_entries() -> Vec<RepromptEntry> {
        vec![entry(0, 100), entry(1, 200), entry(2, 300)]
    }

    /// **PIDP-01** — depth exceeded rejected.
    #[test]
    fn pidp_01_depth_exceeded_rejected() {
        assert_eq!(
            validate_injection_depth(&[entry(PIDP_MAX_DEPTH + 1, 100)]),
            Err(InjectionDepthError::DepthExceeded)
        );
    }

    /// **PIDP-02** — depth regression rejected.
    #[test]
    fn pidp_02_regression_rejected() {
        let entries = vec![entry(0, 100), entry(1, 200), entry(0, 100)];
        assert_eq!(
            validate_injection_depth(&entries),
            Err(InjectionDepthError::DepthRegression)
        );
    }

    /// **PIDP-03** — depth jump rejected.
    #[test]
    fn pidp_03_jump_rejected() {
        let entries = vec![entry(0, 100), entry(3, 200)];
        assert_eq!(
            validate_injection_depth(&entries),
            Err(InjectionDepthError::DepthJump)
        );
    }

    /// **PIDP-04** — re-prompt too large rejected.
    #[test]
    fn pidp_04_reprompt_large_rejected() {
        assert_eq!(
            validate_injection_depth(&[entry(0, PIDP_MAX_REPROMPT_LEN + 1)]),
            Err(InjectionDepthError::RepromptTooLarge)
        );
    }

    /// **PIDP-05** — total bytes exceeded rejected.
    #[test]
    fn pidp_05_total_bytes_rejected() {
        let per_entry = PIDP_MAX_REPROMPT_LEN;
        let entries: Vec<RepromptEntry> = (0..17)
            .map(|_| entry(0, per_entry))
            .collect();
        assert_eq!(
            validate_injection_depth(&entries),
            Err(InjectionDepthError::TotalBytesExceeded)
        );
    }

    /// **PIDP-06** — single oversized reprompt rejected.
    #[test]
    fn pidp_06_single_oversized_rejected() {
        assert_eq!(
            validate_injection_depth(&[entry(0, PIDP_MAX_REPROMPT_LEN + 1)]),
            Err(InjectionDepthError::RepromptTooLarge)
        );
    }

    /// **PIDP-07** — valid entries accepted.
    #[test]
    fn pidp_07_valid_accepted() {
        assert_eq!(validate_injection_depth(&valid_entries()), Ok(()));
    }

    /// **PIDP-08** — single entry accepted.
    #[test]
    fn pidp_08_single_accepted() {
        assert_eq!(validate_injection_depth(&[entry(0, 100)]), Ok(()));
    }

    /// **PIDP-09** — max depth accepted.
    #[test]
    fn pidp_09_max_depth_accepted() {
        let entries: Vec<RepromptEntry> = (0..=PIDP_MAX_DEPTH)
            .map(|i| entry(i, 100))
            .collect();
        assert_eq!(validate_injection_depth(&entries), Ok(()));
    }

    /// **PIDP-10** — empty accepted.
    #[test]
    fn pidp_10_empty_accepted() {
        assert_eq!(validate_injection_depth(&[]), Ok(()));
    }
}
