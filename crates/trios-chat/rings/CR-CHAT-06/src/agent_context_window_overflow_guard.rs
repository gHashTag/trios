//! # CR-CHAT-06 — Agent context window overflow guard (Wave-131 Lane A)
//!
//! AGENT SAFETY — agent context window inputs must not exceed the
//! maximum budget; overflow causes truncation that drops safety-
//! critical instructions.
//!
//! When an agent processes a conversation, all inputs (system prompt,
//! tool results, user messages) must fit within a fixed context window:
//!
//! * **Safety instruction truncation** — if the context overflows,
//!   the oldest messages are dropped first, which may include the
//!   system safety prompt.
//! * **Injection amplification** — an attacker who can cause context
//!   overflow can force the agent to "forget" its safety constraints.
//! * **Budget exhaustion** — each tool call adds context; unbounded
//!   tool chains exhaust the budget, degrading agent behavior.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Total tokens <= `ACWO_MAX_BUDGET`.
//! 2. Entry tokens must be > 0.
//! 3. Entry ID must not be zero.
//! 4. No duplicate entry IDs.
//! 5. Priority must be <= `ACWO_MAX_PRIORITY`.
//! 6. Total entries <= `ACWO_MAX_ENTRIES`.
//!
//! Tests **ACWO-01..10**. Error enum [`ContextOverflowError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CONTEXT-BOUNDED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum context window budget in tokens.
pub const ACWO_MAX_BUDGET: u64 = 128_000;

/// Maximum priority level.
pub const ACWO_MAX_PRIORITY: u8 = 10;

/// Maximum entries per batch.
pub const ACWO_MAX_ENTRIES: usize = 1024;

/// Entry ID length.
pub const ACWO_ENTRY_ID_LEN: usize = 32;

/// A context window entry.
#[derive(Debug, Clone)]
pub struct ContextWindowEntry {
    /// Entry identifier.
    pub entry_id: [u8; ACWO_ENTRY_ID_LEN],
    /// Token count for this entry.
    pub tokens: u64,
    /// Priority level (higher = less likely to be evicted).
    pub priority: u8,
}

/// All ways context window overflow validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContextOverflowError {
    /// Total tokens exceed budget.
    BudgetExceeded { total: u64, max: u64 },
    /// Zero token count.
    ZeroTokens(usize),
    /// Zero entry ID.
    ZeroEntryId(usize),
    /// Duplicate entry ID.
    DuplicateEntryId { idx: usize },
    /// Priority exceeds maximum.
    PriorityTooHigh { idx: usize, got: u8, max: u8 },
    /// Too many entries.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent context window overflow.
pub fn validate_context_overflow(
    entries: &[ContextWindowEntry],
) -> Result<(), ContextOverflowError> {
    if entries.len() > ACWO_MAX_ENTRIES {
        return Err(ContextOverflowError::TooMany {
            got: entries.len(),
            max: ACWO_MAX_ENTRIES,
        });
    }
    let mut seen: BTreeSet<[u8; ACWO_ENTRY_ID_LEN]> = BTreeSet::new();
    let mut total: u64 = 0;
    for (i, e) in entries.iter().enumerate() {
        if e.entry_id == [0u8; ACWO_ENTRY_ID_LEN] {
            return Err(ContextOverflowError::ZeroEntryId(i));
        }
        if e.tokens == 0 {
            return Err(ContextOverflowError::ZeroTokens(i));
        }
        if e.priority > ACWO_MAX_PRIORITY {
            return Err(ContextOverflowError::PriorityTooHigh {
                idx: i,
                got: e.priority,
                max: ACWO_MAX_PRIORITY,
            });
        }
        if !seen.insert(e.entry_id) {
            return Err(ContextOverflowError::DuplicateEntryId { idx: i });
        }
        total = total.saturating_add(e.tokens);
    }
    if total > ACWO_MAX_BUDGET {
        return Err(ContextOverflowError::BudgetExceeded {
            total,
            max: ACWO_MAX_BUDGET,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eid(byte: u8) -> [u8; ACWO_ENTRY_ID_LEN] {
        [byte; ACWO_ENTRY_ID_LEN]
    }

    fn entry(id: u8, tokens: u64, priority: u8) -> ContextWindowEntry {
        ContextWindowEntry { entry_id: eid(id), tokens, priority }
    }

    fn valid_entries() -> Vec<ContextWindowEntry> {
        vec![
            entry(0x01, 1000, 10),
            entry(0x02, 5000, 5),
            entry(0x03, 2000, 3),
        ]
    }

    /// **ACWO-01** — budget exceeded rejected.
    #[test]
    fn acwo_01_budget_exceeded_rejected() {
        let es = vec![entry(0x01, ACWO_MAX_BUDGET + 1, 5)];
        assert_eq!(
            validate_context_overflow(&es),
            Err(ContextOverflowError::BudgetExceeded {
                total: ACWO_MAX_BUDGET + 1,
                max: ACWO_MAX_BUDGET,
            })
        );
    }

    /// **ACWO-02** — zero tokens rejected.
    #[test]
    fn acwo_02_zero_tokens_rejected() {
        let e = ContextWindowEntry { entry_id: eid(0x01), tokens: 0, priority: 5 };
        assert_eq!(
            validate_context_overflow(&[e]),
            Err(ContextOverflowError::ZeroTokens(0))
        );
    }

    /// **ACWO-03** — zero entry ID rejected.
    #[test]
    fn acwo_03_zero_id_rejected() {
        let e = ContextWindowEntry { entry_id: [0u8; ACWO_ENTRY_ID_LEN], tokens: 100, priority: 5 };
        assert_eq!(
            validate_context_overflow(&[e]),
            Err(ContextOverflowError::ZeroEntryId(0))
        );
    }

    /// **ACWO-04** — duplicate entry ID rejected.
    #[test]
    fn acwo_04_duplicate_rejected() {
        let es = vec![
            entry(0x01, 100, 5),
            entry(0x01, 200, 5),
        ];
        assert_eq!(
            validate_context_overflow(&es),
            Err(ContextOverflowError::DuplicateEntryId { idx: 1 })
        );
    }

    /// **ACWO-05** — priority too high rejected.
    #[test]
    fn acwo_05_priority_too_high_rejected() {
        let e = ContextWindowEntry { entry_id: eid(0x01), tokens: 100, priority: ACWO_MAX_PRIORITY + 1 };
        assert_eq!(
            validate_context_overflow(&[e]),
            Err(ContextOverflowError::PriorityTooHigh {
                idx: 0,
                got: ACWO_MAX_PRIORITY + 1,
                max: ACWO_MAX_PRIORITY,
            })
        );
    }

    /// **ACWO-06** — too many rejected.
    #[test]
    fn acwo_06_too_many_rejected() {
        let es: Vec<ContextWindowEntry> = (0..=ACWO_MAX_ENTRIES)
            .map(|i| {
                let mut id = [0u8; ACWO_ENTRY_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                ContextWindowEntry { entry_id: id, tokens: 100, priority: 5 }
            })
            .collect();
        assert_eq!(
            validate_context_overflow(&es),
            Err(ContextOverflowError::TooMany {
                got: ACWO_MAX_ENTRIES + 1,
                max: ACWO_MAX_ENTRIES,
            })
        );
    }

    /// **ACWO-07** — valid accepted.
    #[test]
    fn acwo_07_valid_accepted() {
        assert_eq!(validate_context_overflow(&valid_entries()), Ok(()));
    }

    /// **ACWO-08** — empty accepted.
    #[test]
    fn acwo_08_empty_accepted() {
        assert_eq!(validate_context_overflow(&[]), Ok(()));
    }

    /// **ACWO-09** — boundary budget accepted.
    #[test]
    fn acwo_09_boundary_budget_accepted() {
        let es = vec![entry(0x01, ACWO_MAX_BUDGET, 5)];
        assert_eq!(validate_context_overflow(&es), Ok(()));
    }

    /// **ACWO-10** — many small entries accepted.
    #[test]
    fn acwo_10_many_small_accepted() {
        let es: Vec<ContextWindowEntry> = (0..100u64)
            .map(|i| {
                let mut id = [0u8; ACWO_ENTRY_ID_LEN];
                id[0..8].copy_from_slice(&(i + 1).to_be_bytes());
                ContextWindowEntry { entry_id: id, tokens: 1000, priority: 5 }
            })
            .collect();
        assert_eq!(validate_context_overflow(&es), Ok(()));
    }
}
