//! # CR-CHAT-06 — Capability scope escalation guard (Wave-58 Lane B)
//!
//! БЕЗОПАСНОСТЬ АГЕНТА — scope не расширяется, R-CHAT-6.
//!
//! Capability token определяет, какие tools может вызывать агент.
//! Атакующий пытается расширить scope в ходе сессии:
//!
//! * **Добавить tool** — получить доступ к filesystem после initial scope.
//! * **Повысить privilege** — readonly → readwrite.
//! * **Обойти expiry** — продлить TTL token'а.
//!
//! 1. New scope ⊆ old scope (no expansion).
//! 2. Scope version monotonic.
//! 3. TTL не увеличивается.
//! 4. No new tool IDs.
//! 5. Max scope changes ≤ `CSEG_MAX_CHANGES`.
//! 6. Scope change requires re-signing.
//!
//! Tests **CSEG-01..10**. Error enum [`ScopeEscalationError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · SCOPE-ESCALATION`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum scope changes per session.
pub const CSEG_MAX_CHANGES: usize = 16;

/// All ways scope escalation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScopeEscalationError {
    /// Scope expanded (new has tools not in old).
    ScopeExpanded,
    /// Version not monotonic.
    VersionNotMonotonic,
    /// TTL increased.
    TtlIncreased,
    /// Too many changes.
    TooManyChanges,
    /// Unsigned scope change.
    UnsignedChange,
    /// Empty scope not allowed.
    EmptyScope,
}

/// A scope snapshot.
#[derive(Debug, Clone)]
pub struct ScopeSnapshot {
    /// Version counter.
    pub version: u32,
    /// Allowed tool IDs.
    pub tool_ids: BTreeSet<u8>,
    /// TTL in seconds.
    pub ttl_secs: u64,
    /// Whether the scope is signed.
    pub signed: bool,
}

/// `[VERIFIED]` Validate a scope transition (old → new).
pub fn validate_scope_transition(
    old: &ScopeSnapshot,
    new: &ScopeSnapshot,
) -> Result<(), ScopeEscalationError> {
    if new.tool_ids.is_empty() {
        return Err(ScopeEscalationError::EmptyScope);
    }
    if !new.signed {
        return Err(ScopeEscalationError::UnsignedChange);
    }
    if new.version <= old.version {
        return Err(ScopeEscalationError::VersionNotMonotonic);
    }
    if new.ttl_secs > old.ttl_secs {
        return Err(ScopeEscalationError::TtlIncreased);
    }
    for tool in &new.tool_ids {
        if !old.tool_ids.contains(tool) {
            return Err(ScopeEscalationError::ScopeExpanded);
        }
    }
    Ok(())
}

/// `[VERIFIED]` Validate a sequence of scope changes.
pub fn validate_scope_history(
    history: &[ScopeSnapshot],
) -> Result<(), ScopeEscalationError> {
    if history.len() > CSEG_MAX_CHANGES {
        return Err(ScopeEscalationError::TooManyChanges);
    }
    for w in history.windows(2) {
        validate_scope_transition(&w[0], &w[1])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(version: u32, tools: &[u8], ttl: u64, signed: bool) -> ScopeSnapshot {
        ScopeSnapshot {
            version,
            tool_ids: tools.iter().copied().collect(),
            ttl_secs: ttl,
            signed,
        }
    }

    /// **CSEG-01** — scope expanded rejected.
    #[test]
    fn cseg_01_expanded_rejected() {
        let old = scope(1, &[1, 2, 3], 3600, true);
        let new = scope(2, &[1, 2, 3, 4], 3600, true);
        assert_eq!(
            validate_scope_transition(&old, &new),
            Err(ScopeEscalationError::ScopeExpanded)
        );
    }

    /// **CSEG-02** — version not monotonic rejected.
    #[test]
    fn cseg_02_version_rejected() {
        let old = scope(2, &[1, 2], 3600, true);
        let new = scope(1, &[1], 3600, true);
        assert_eq!(
            validate_scope_transition(&old, &new),
            Err(ScopeEscalationError::VersionNotMonotonic)
        );
    }

    /// **CSEG-03** — TTL increased rejected.
    #[test]
    fn cseg_03_ttl_rejected() {
        let old = scope(1, &[1, 2], 3600, true);
        let new = scope(2, &[1], 7200, true);
        assert_eq!(
            validate_scope_transition(&old, &new),
            Err(ScopeEscalationError::TtlIncreased)
        );
    }

    /// **CSEG-04** — unsigned rejected.
    #[test]
    fn cseg_04_unsigned_rejected() {
        let old = scope(1, &[1, 2], 3600, true);
        let new = scope(2, &[1], 3600, false);
        assert_eq!(
            validate_scope_transition(&old, &new),
            Err(ScopeEscalationError::UnsignedChange)
        );
    }

    /// **CSEG-05** — empty scope rejected.
    #[test]
    fn cseg_05_empty_rejected() {
        let old = scope(1, &[1], 3600, true);
        let new = scope(2, &[], 3600, true);
        assert_eq!(
            validate_scope_transition(&old, &new),
            Err(ScopeEscalationError::EmptyScope)
        );
    }

    /// **CSEG-06** — valid narrowing accepted.
    #[test]
    fn cseg_06_narrowing_accepted() {
        let old = scope(1, &[1, 2, 3], 3600, true);
        let new = scope(2, &[1, 2], 3600, true);
        assert_eq!(validate_scope_transition(&old, &new), Ok(()));
    }

    /// **CSEG-07** — TTL decrease accepted.
    #[test]
    fn cseg_07_ttl_decrease_accepted() {
        let old = scope(1, &[1, 2], 3600, true);
        let new = scope(2, &[1], 1800, true);
        assert_eq!(validate_scope_transition(&old, &new), Ok(()));
    }

    /// **CSEG-08** — history accepted.
    #[test]
    fn cseg_08_history_accepted() {
        let h = vec![
            scope(1, &[1, 2, 3], 3600, true),
            scope(2, &[1, 2], 1800, true),
            scope(3, &[1], 900, true),
        ];
        assert_eq!(validate_scope_history(&h), Ok(()));
    }

    /// **CSEG-09** — same TTL accepted.
    #[test]
    fn cseg_09_same_ttl_accepted() {
        let old = scope(1, &[1, 2], 3600, true);
        let new = scope(2, &[1], 3600, true);
        assert_eq!(validate_scope_transition(&old, &new), Ok(()));
    }

    /// **CSEG-10** — too many changes rejected.
    #[test]
    fn cseg_10_too_many_rejected() {
        let h: Vec<ScopeSnapshot> = (0..=CSEG_MAX_CHANGES)
            .map(|i| scope(i as u32 + 1, &[1], 3600, true))
            .collect();
        assert_eq!(
            validate_scope_history(&h),
            Err(ScopeEscalationError::TooManyChanges)
        );
    }
}
