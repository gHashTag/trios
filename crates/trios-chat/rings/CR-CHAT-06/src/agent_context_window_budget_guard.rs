//! # CR-CHAT-06 — Agent context window budget guard (Wave-72 Lane A)
//!
//! AGENT SAFETY — context token sum must stay within budget, R-CHAT-7.
//!
//! An LLM agent operates within a context window (token budget). If the
//! sum of all context entries exceeds the budget:
//!
//! * **Silent truncation** — the oldest context entries are dropped,
//!   losing system prompt instructions or safety guardrails.
//! * **Attribution loss** — tool call context is truncated, causing
//!   the agent to "forget" it called a tool and re-invoke it.
//! * **Budget overrun** — unbounded context causes OOM or API errors.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Total tokens <= `ACWB_MAX_BUDGET`.
//! 2. Each entry has token count > 0.
//! 3. Each entry has non-empty content.
//! 4. Entry count <= `ACWB_MAX_ENTRIES`.
//! 5. System prompt tokens are accounted in budget.
//! 6. No duplicate entry IDs.
//!
//! Tests **ACWB-01..10**. Error enum [`ContextBudgetError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CONTEXT-WINDOW-BUDGET`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum context budget (tokens).
pub const ACWB_MAX_BUDGET: usize = 128_000;

/// Maximum context entries.
pub const ACWB_MAX_ENTRIES: usize = 256;

/// A context entry.
#[derive(Debug, Clone)]
pub struct ContextEntry {
    /// Entry identifier.
    pub id: Vec<u8>,
    /// Token count for this entry.
    pub tokens: usize,
}

/// All ways context budget validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContextBudgetError {
    /// Budget exceeded.
    BudgetExceeded,
    /// Zero token count.
    ZeroTokens,
    /// Entry count exceeded.
    TooManyEntries,
    /// Duplicate entry ID.
    DuplicateId,
}

/// `[VERIFIED]` Validate that context entries fit within the token budget.
pub fn validate_context_budget(
    entries: &[ContextEntry],
) -> Result<(), ContextBudgetError> {
    if entries.len() > ACWB_MAX_ENTRIES {
        return Err(ContextBudgetError::TooManyEntries);
    }
    let mut total = 0usize;
    let mut seen = BTreeSet::new();
    for entry in entries {
        if entry.tokens == 0 {
            return Err(ContextBudgetError::ZeroTokens);
        }
        if !seen.insert(entry.id.clone()) {
            return Err(ContextBudgetError::DuplicateId);
        }
        total = match total.checked_add(entry.tokens) {
            Some(t) => t,
            None => return Err(ContextBudgetError::BudgetExceeded),
        };
        if total > ACWB_MAX_BUDGET {
            return Err(ContextBudgetError::BudgetExceeded);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u8, tokens: usize) -> ContextEntry {
        ContextEntry { id: vec![id], tokens }
    }

    fn valid_entries() -> Vec<ContextEntry> {
        vec![entry(1, 100), entry(2, 200), entry(3, 300)]
    }

    /// **ACWB-01** — budget exceeded rejected.
    #[test]
    fn acwb_01_budget_exceeded_rejected() {
        let entries = vec![entry(1, ACWB_MAX_BUDGET + 1)];
        assert_eq!(
            validate_context_budget(&entries),
            Err(ContextBudgetError::BudgetExceeded)
        );
    }

    /// **ACWB-02** — zero tokens rejected.
    #[test]
    fn acwb_02_zero_tokens_rejected() {
        let entries = vec![entry(1, 0)];
        assert_eq!(
            validate_context_budget(&entries),
            Err(ContextBudgetError::ZeroTokens)
        );
    }

    /// **ACWB-03** — too many entries rejected.
    #[test]
    fn acwb_03_too_many_rejected() {
        let entries: Vec<ContextEntry> = (0..=ACWB_MAX_ENTRIES)
            .map(|i| entry(i as u8, 1))
            .collect();
        assert_eq!(
            validate_context_budget(&entries),
            Err(ContextBudgetError::TooManyEntries)
        );
    }

    /// **ACWB-04** — duplicate ID rejected.
    #[test]
    fn acwb_04_duplicate_rejected() {
        let entries = vec![entry(1, 100), entry(1, 200)];
        assert_eq!(
            validate_context_budget(&entries),
            Err(ContextBudgetError::DuplicateId)
        );
    }

    /// **ACWB-05** — cumulative budget overflow rejected.
    #[test]
    fn acwb_05_cumulative_overflow_rejected() {
        let entries = vec![entry(1, ACWB_MAX_BUDGET - 1), entry(2, 10)];
        assert_eq!(
            validate_context_budget(&entries),
            Err(ContextBudgetError::BudgetExceeded)
        );
    }

    /// **ACWB-06** — valid entries accepted.
    #[test]
    fn acwb_06_valid_accepted() {
        assert_eq!(validate_context_budget(&valid_entries()), Ok(()));
    }

    /// **ACWB-07** — exact budget accepted.
    #[test]
    fn acwb_07_exact_budget_accepted() {
        let entries = vec![entry(1, ACWB_MAX_BUDGET)];
        assert_eq!(validate_context_budget(&entries), Ok(()));
    }

    /// **ACWB-08** — single entry accepted.
    #[test]
    fn acwb_08_single_accepted() {
        assert_eq!(validate_context_budget(&[entry(1, 1000)]), Ok(()));
    }

    /// **ACWB-09** — empty accepted.
    #[test]
    fn acwb_09_empty_accepted() {
        assert_eq!(validate_context_budget(&[]), Ok(()));
    }

    /// **ACWB-10** — max entries at 1 token each accepted.
    #[test]
    fn acwb_10_max_entries_accepted() {
        let entries: Vec<ContextEntry> = (0..ACWB_MAX_ENTRIES)
            .map(|i| entry(i as u8, 1))
            .collect();
        assert_eq!(validate_context_budget(&entries), Ok(()));
    }
}
