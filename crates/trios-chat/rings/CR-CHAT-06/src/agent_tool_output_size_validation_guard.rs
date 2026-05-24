//! # CR-CHAT-06 — Agent tool output size validation guard (Wave-97 Lane A)
//!
//! AGENT SAFETY — accumulated tool output size must be bounded,
//! R-CHAT-7.
//!
//! Each tool call produces output. Across a session, the accumulated
//! output size must be bounded. Without a limit:
//!
//! * **Memory exhaustion** — a compromised agent calls tools that
//!   return massive outputs, accumulating until OOM.
//! * **Context overflow** — tool outputs feed into the agent's context
//!   window; oversized accumulated output exceeds the model's capacity.
//! * **Disk exhaustion** — persisted tool outputs fill storage.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Accumulated size <= `ATOS_MAX_ACCUMULATED`.
//! 2. Single output size <= `ATOS_MAX_SINGLE`.
//! 3. Output count <= `ATOS_MAX_OUTPUTS`.
//! 4. Output size must be > 0.
//! 5. Tool name must be non-empty.
//! 6. No duplicate output IDs.
//!
//! Tests **ATOS-01..10**. Error enum [`OutputSizeError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * OUTPUT-SIZE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum accumulated output size.
pub const ATOS_MAX_ACCUMULATED: usize = 10_485_760;

/// Maximum single output size.
pub const ATOS_MAX_SINGLE: usize = 1_048_576;

/// Maximum outputs per session.
pub const ATOS_MAX_OUTPUTS: usize = 1024;

/// A tool output record.
#[derive(Debug, Clone)]
pub struct ToolOutputRecord {
    /// Output ID.
    pub id: u64,
    /// Tool name.
    pub tool: String,
    /// Output size in bytes.
    pub size: usize,
}

/// All ways output size validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputSizeError {
    /// Accumulated size exceeded.
    AccumulatedExceeded { total: usize, max: usize },
    /// Single output too large.
    SingleTooLarge { size: usize, max: usize },
    /// Too many outputs.
    TooManyOutputs,
    /// Zero size.
    ZeroSize(u64),
    /// Empty tool name.
    EmptyToolName,
    /// Duplicate ID.
    DuplicateId(u64),
}

/// `[VERIFIED]` Validate accumulated tool output sizes.
pub fn validate_tool_output_sizes(
    outputs: &[ToolOutputRecord],
) -> Result<(), OutputSizeError> {
    if outputs.len() > ATOS_MAX_OUTPUTS {
        return Err(OutputSizeError::TooManyOutputs);
    }
    let mut total = 0usize;
    let mut seen = BTreeSet::new();
    for o in outputs {
        if o.tool.is_empty() {
            return Err(OutputSizeError::EmptyToolName);
        }
        if o.size == 0 {
            return Err(OutputSizeError::ZeroSize(o.id));
        }
        if o.size > ATOS_MAX_SINGLE {
            return Err(OutputSizeError::SingleTooLarge { size: o.size, max: ATOS_MAX_SINGLE });
        }
        if !seen.insert(o.id) {
            return Err(OutputSizeError::DuplicateId(o.id));
        }
        total += o.size;
        if total > ATOS_MAX_ACCUMULATED {
            return Err(OutputSizeError::AccumulatedExceeded { total, max: ATOS_MAX_ACCUMULATED });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(id: u64, tool: &str, size: usize) -> ToolOutputRecord {
        ToolOutputRecord { id, tool: tool.to_string(), size }
    }

    fn valid_outputs() -> Vec<ToolOutputRecord> {
        vec![out(1, "read", 100), out(2, "search", 200)]
    }

    /// **ATOS-01** — accumulated exceeded rejected.
    #[test]
    fn atos_01_accumulated_rejected() {
        let size = ATOS_MAX_SINGLE;
        let count = ATOS_MAX_ACCUMULATED / size + 1;
        let outputs: Vec<ToolOutputRecord> = (0..count as u64)
            .map(|i| out(i, "tool", size))
            .collect();
        assert!(matches!(
            validate_tool_output_sizes(&outputs),
            Err(OutputSizeError::AccumulatedExceeded { .. })
        ));
    }

    /// **ATOS-02** — single too large rejected.
    #[test]
    fn atos_02_single_too_large_rejected() {
        let o = out(1, "tool", ATOS_MAX_SINGLE + 1);
        assert_eq!(
            validate_tool_output_sizes(&[o]),
            Err(OutputSizeError::SingleTooLarge { size: ATOS_MAX_SINGLE + 1, max: ATOS_MAX_SINGLE })
        );
    }

    /// **ATOS-03** — too many outputs rejected.
    #[test]
    fn atos_03_too_many_rejected() {
        let outputs: Vec<ToolOutputRecord> = (0..=ATOS_MAX_OUTPUTS as u64)
            .map(|i| out(i, "tool", 10))
            .collect();
        assert_eq!(validate_tool_output_sizes(&outputs), Err(OutputSizeError::TooManyOutputs));
    }

    /// **ATOS-04** — zero size rejected.
    #[test]
    fn atos_04_zero_size_rejected() {
        let o = out(1, "tool", 0);
        assert_eq!(validate_tool_output_sizes(&[o]), Err(OutputSizeError::ZeroSize(1)));
    }

    /// **ATOS-05** — empty tool name rejected.
    #[test]
    fn atos_05_empty_tool_rejected() {
        let o = ToolOutputRecord { id: 1, tool: String::new(), size: 100 };
        assert_eq!(validate_tool_output_sizes(&[o]), Err(OutputSizeError::EmptyToolName));
    }

    /// **ATOS-06** — duplicate ID rejected.
    #[test]
    fn atos_06_duplicate_rejected() {
        let os = vec![out(1, "tool", 100), out(1, "tool", 200)];
        assert_eq!(validate_tool_output_sizes(&os), Err(OutputSizeError::DuplicateId(1)));
    }

    /// **ATOS-07** — valid outputs accepted.
    #[test]
    fn atos_07_valid_accepted() {
        assert_eq!(validate_tool_output_sizes(&valid_outputs()), Ok(()));
    }

    /// **ATOS-08** — empty accepted.
    #[test]
    fn atos_08_empty_accepted() {
        assert_eq!(validate_tool_output_sizes(&[]), Ok(()));
    }

    /// **ATOS-09** — single at max accepted.
    #[test]
    fn atos_09_single_max_accepted() {
        assert_eq!(validate_tool_output_sizes(&[out(1, "tool", ATOS_MAX_SINGLE)]), Ok(()));
    }

    /// **ATOS-10** — many small outputs accepted.
    #[test]
    fn atos_10_many_small_accepted() {
        let outputs: Vec<ToolOutputRecord> = (0..100)
            .map(|i| out(i, "tool", 1000))
            .collect();
        assert_eq!(validate_tool_output_sizes(&outputs), Ok(()));
    }
}
