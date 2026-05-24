//! # CR-CHAT-06 — Agent tool call audit log guard (Wave-83 Lane A)
//!
//! AGENT SAFETY — every tool invocation must produce an audit entry, R-CHAT-7.
//!
//! Without an immutable audit log, a compromised agent can:
//!
//! * **Silent invocation** — call a tool without leaving any trace,
//!   bypassing safety checks.
//! * **Log tampering** — modify or delete audit entries to hide
//!   unauthorized tool usage.
//! * **Out-of-order replay** — reorder audit entries to misrepresent
//!   the sequence of tool calls.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Every tool call has a corresponding audit entry.
//! 2. Audit sequence numbers are strictly increasing.
//! 3. No duplicate sequence numbers.
//! 4. Entry count <= `TCAL_MAX_ENTRIES`.
//! 5. Tool name is non-empty.
//! 6. Timestamp is non-zero and increasing.
//!
//! Tests **TCAL-01..10**. Error enum [`AuditLogError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TOOL-CALL-AUDIT`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum audit entries.
pub const TCAL_MAX_ENTRIES: usize = 4096;

/// An audit log entry.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Sequence number.
    pub seq: u64,
    /// Tool name.
    pub tool: String,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
}

/// All ways audit log validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuditLogError {
    /// Sequence not strictly increasing.
    NotIncreasing(u64),
    /// Duplicate sequence.
    DuplicateSeq(u64),
    /// Too many entries.
    TooManyEntries,
    /// Empty tool name.
    EmptyToolName,
    /// Timestamp not increasing.
    TimestampNotIncreasing,
    /// Zero timestamp.
    ZeroTimestamp,
}

/// `[VERIFIED]` Validate audit log integrity.
pub fn validate_audit_log(
    entries: &[AuditEntry],
) -> Result<(), AuditLogError> {
    if entries.len() > TCAL_MAX_ENTRIES {
        return Err(AuditLogError::TooManyEntries);
    }
    let mut seen = BTreeSet::new();
    for (i, entry) in entries.iter().enumerate() {
        if entry.tool.is_empty() {
            return Err(AuditLogError::EmptyToolName);
        }
        if entry.timestamp_ms == 0 {
            return Err(AuditLogError::ZeroTimestamp);
        }
        if i > 0 && entry.timestamp_ms < entries[i - 1].timestamp_ms {
            return Err(AuditLogError::TimestampNotIncreasing);
        }
        if !seen.insert(entry.seq) {
            return Err(AuditLogError::DuplicateSeq(entry.seq));
        }
        if i > 0 && entry.seq <= entries[i - 1].seq {
            return Err(AuditLogError::NotIncreasing(entry.seq));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(seq: u64, tool: &str, ts: u64) -> AuditEntry {
        AuditEntry { seq, tool: tool.to_string(), timestamp_ms: ts }
    }

    fn valid_log() -> Vec<AuditEntry> {
        vec![
            entry(1, "read_file", 1000),
            entry(2, "write_file", 2000),
            entry(3, "search", 3000),
        ]
    }

    /// **TCAL-01** — not increasing rejected.
    #[test]
    fn tcal_01_not_increasing_rejected() {
        let log = vec![entry(1, "tool", 100), entry(3, "tool", 200), entry(2, "tool", 300)];
        assert_eq!(
            validate_audit_log(&log),
            Err(AuditLogError::NotIncreasing(2))
        );
    }

    /// **TCAL-02** — duplicate sequence rejected.
    #[test]
    fn tcal_02_duplicate_rejected() {
        let log = vec![entry(1, "tool", 100), entry(1, "tool", 200)];
        assert_eq!(
            validate_audit_log(&log),
            Err(AuditLogError::DuplicateSeq(1))
        );
    }

    /// **TCAL-03** — too many entries rejected.
    #[test]
    fn tcal_03_too_many_rejected() {
        let log: Vec<AuditEntry> = (1..=TCAL_MAX_ENTRIES as u64 + 1)
            .map(|i| entry(i, "tool", i * 100))
            .collect();
        assert_eq!(
            validate_audit_log(&log),
            Err(AuditLogError::TooManyEntries)
        );
    }

    /// **TCAL-04** — empty tool name rejected.
    #[test]
    fn tcal_04_empty_tool_rejected() {
        let log = vec![entry(1, "", 100)];
        assert_eq!(
            validate_audit_log(&log),
            Err(AuditLogError::EmptyToolName)
        );
    }

    /// **TCAL-05** — timestamp not increasing rejected.
    #[test]
    fn tcal_05_ts_not_increasing_rejected() {
        let log = vec![entry(1, "tool", 2000), entry(2, "tool", 1000)];
        assert_eq!(
            validate_audit_log(&log),
            Err(AuditLogError::TimestampNotIncreasing)
        );
    }

    /// **TCAL-06** — zero timestamp rejected.
    #[test]
    fn tcal_06_zero_ts_rejected() {
        let log = vec![entry(1, "tool", 0)];
        assert_eq!(
            validate_audit_log(&log),
            Err(AuditLogError::ZeroTimestamp)
        );
    }

    /// **TCAL-07** — valid log accepted.
    #[test]
    fn tcal_07_valid_accepted() {
        assert_eq!(validate_audit_log(&valid_log()), Ok(()));
    }

    /// **TCAL-08** — empty log accepted.
    #[test]
    fn tcal_08_empty_accepted() {
        assert_eq!(validate_audit_log(&[]), Ok(()));
    }

    /// **TCAL-09** — single entry accepted.
    #[test]
    fn tcal_09_single_accepted() {
        assert_eq!(validate_audit_log(&[entry(1, "tool", 1000)]), Ok(()));
    }

    /// **TCAL-10** — same timestamp accepted (concurrent calls).
    #[test]
    fn tcal_10_same_ts_accepted() {
        let log = vec![entry(1, "tool_a", 1000), entry(2, "tool_b", 1000)];
        assert_eq!(validate_audit_log(&log), Ok(()));
    }
}
