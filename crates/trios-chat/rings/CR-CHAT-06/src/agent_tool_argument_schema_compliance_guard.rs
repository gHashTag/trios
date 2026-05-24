//! # CR-CHAT-06 — Agent tool argument schema compliance guard (Wave-145 Lane A)
//!
//! AGENT SAFETY — tool call arguments must conform to declared
//! schema; schema violations enable injection through malformed inputs.
//!
//! Each tool declares a schema (expected argument types, counts,
//! ranges). If tool call arguments violate the schema:
//!
//! * **Type confusion** — passing a string where a number is
//!   expected can trigger unexpected code paths.
//! * **Injection** — malformed arguments can exploit parsing bugs
//!   in tool implementations.
//! * **Resource abuse** — oversized arguments can consume excessive
//!   memory or trigger expensive computations.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Argument count <= `TASC_MAX_ARGS`.
//! 2. Argument count >= `TASC_MIN_ARGS`.
//! 3. Tool ID must not be zero.
//! 4. No duplicate tool IDs.
//! 5. Each arg length <= `TASC_MAX_ARG_LEN`.
//! 6. Batch size <= `TASC_MAX_CALLS`.
//!
//! Tests **TASC-01..10**. Error enum [`SchemaComplianceError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SCHEMA-VALID`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum arguments per call.
pub const TASC_MAX_ARGS: usize = 32;

/// Minimum arguments per call.
pub const TASC_MIN_ARGS: usize = 1;

/// Maximum argument length in bytes.
pub const TASC_MAX_ARG_LEN: usize = 4096;

/// Maximum calls per batch.
pub const TASC_MAX_CALLS: usize = 256;

/// Tool ID length.
pub const TASC_TOOL_ID_LEN: usize = 16;

/// A tool call argument schema record.
#[derive(Debug, Clone)]
pub struct ToolArgRecord {
    /// Tool identifier.
    pub tool_id: [u8; TASC_TOOL_ID_LEN],
    /// Number of arguments.
    pub arg_count: usize,
    /// Maximum argument length in this call.
    pub max_arg_len: usize,
}

/// All ways schema compliance validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaComplianceError {
    /// Too many arguments.
    TooManyArgs { idx: usize, got: usize, max: usize },
    /// Too few arguments.
    TooFewArgs { idx: usize, got: usize, min: usize },
    /// Zero tool ID.
    ZeroToolId(usize),
    /// Duplicate tool ID.
    DuplicateToolId { idx: usize },
    /// Argument too long.
    ArgTooLong { idx: usize, got: usize, max: usize },
    /// Too many calls.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent tool argument schema compliance.
pub fn validate_schema_compliance(
    calls: &[ToolArgRecord],
) -> Result<(), SchemaComplianceError> {
    if calls.len() > TASC_MAX_CALLS {
        return Err(SchemaComplianceError::TooMany {
            got: calls.len(),
            max: TASC_MAX_CALLS,
        });
    }
    let mut seen: BTreeSet<[u8; TASC_TOOL_ID_LEN]> = BTreeSet::new();
    for (i, c) in calls.iter().enumerate() {
        if c.tool_id == [0u8; TASC_TOOL_ID_LEN] {
            return Err(SchemaComplianceError::ZeroToolId(i));
        }
        if !seen.insert(c.tool_id) {
            return Err(SchemaComplianceError::DuplicateToolId { idx: i });
        }
        if c.arg_count < TASC_MIN_ARGS {
            return Err(SchemaComplianceError::TooFewArgs {
                idx: i,
                got: c.arg_count,
                min: TASC_MIN_ARGS,
            });
        }
        if c.arg_count > TASC_MAX_ARGS {
            return Err(SchemaComplianceError::TooManyArgs {
                idx: i,
                got: c.arg_count,
                max: TASC_MAX_ARGS,
            });
        }
        if c.max_arg_len > TASC_MAX_ARG_LEN {
            return Err(SchemaComplianceError::ArgTooLong {
                idx: i,
                got: c.max_arg_len,
                max: TASC_MAX_ARG_LEN,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tid(byte: u8) -> [u8; TASC_TOOL_ID_LEN] {
        [byte; TASC_TOOL_ID_LEN]
    }

    fn call(id: u8, args: usize, max_len: usize) -> ToolArgRecord {
        ToolArgRecord { tool_id: tid(id), arg_count: args, max_arg_len: max_len }
    }

    fn valid_calls() -> Vec<ToolArgRecord> {
        vec![
            call(0x01, 3, 256),
            call(0x02, 5, 1024),
        ]
    }

    /// **TASC-01** — too many args rejected.
    #[test]
    fn tasc_01_too_many_args_rejected() {
        let c = call(0x01, TASC_MAX_ARGS + 1, 256);
        assert_eq!(
            validate_schema_compliance(&[c]),
            Err(SchemaComplianceError::TooManyArgs {
                idx: 0,
                got: TASC_MAX_ARGS + 1,
                max: TASC_MAX_ARGS,
            })
        );
    }

    /// **TASC-02** — too few args rejected.
    #[test]
    fn tasc_02_too_few_args_rejected() {
        let c = call(0x01, TASC_MIN_ARGS - 1, 256);
        assert_eq!(
            validate_schema_compliance(&[c]),
            Err(SchemaComplianceError::TooFewArgs {
                idx: 0,
                got: TASC_MIN_ARGS - 1,
                min: TASC_MIN_ARGS,
            })
        );
    }

    /// **TASC-03** — zero tool ID rejected.
    #[test]
    fn tasc_03_zero_tool_rejected() {
        let c = ToolArgRecord { tool_id: [0u8; TASC_TOOL_ID_LEN], arg_count: 3, max_arg_len: 256 };
        assert_eq!(
            validate_schema_compliance(&[c]),
            Err(SchemaComplianceError::ZeroToolId(0))
        );
    }

    /// **TASC-04** — duplicate tool ID rejected.
    #[test]
    fn tasc_04_duplicate_rejected() {
        let cs = vec![
            call(0x01, 3, 256),
            call(0x01, 5, 1024),
        ];
        assert_eq!(
            validate_schema_compliance(&cs),
            Err(SchemaComplianceError::DuplicateToolId { idx: 1 })
        );
    }

    /// **TASC-05** — arg too long rejected.
    #[test]
    fn tasc_05_arg_too_long_rejected() {
        let c = call(0x01, 3, TASC_MAX_ARG_LEN + 1);
        assert_eq!(
            validate_schema_compliance(&[c]),
            Err(SchemaComplianceError::ArgTooLong {
                idx: 0,
                got: TASC_MAX_ARG_LEN + 1,
                max: TASC_MAX_ARG_LEN,
            })
        );
    }

    /// **TASC-06** — too many calls rejected.
    #[test]
    fn tasc_06_too_many_rejected() {
        let cs: Vec<ToolArgRecord> = (0..=TASC_MAX_CALLS)
            .map(|i| {
                let mut id = [0u8; TASC_TOOL_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                ToolArgRecord { tool_id: id, arg_count: 1, max_arg_len: 100 }
            })
            .collect();
        assert_eq!(
            validate_schema_compliance(&cs),
            Err(SchemaComplianceError::TooMany {
                got: TASC_MAX_CALLS + 1,
                max: TASC_MAX_CALLS,
            })
        );
    }

    /// **TASC-07** — valid accepted.
    #[test]
    fn tasc_07_valid_accepted() {
        assert_eq!(validate_schema_compliance(&valid_calls()), Ok(()));
    }

    /// **TASC-08** — empty accepted.
    #[test]
    fn tasc_08_empty_accepted() {
        assert_eq!(validate_schema_compliance(&[]), Ok(()));
    }

    /// **TASC-09** — boundary args accepted.
    #[test]
    fn tasc_09_boundary_args_accepted() {
        let c = call(0x01, TASC_MAX_ARGS, TASC_MAX_ARG_LEN);
        assert_eq!(validate_schema_compliance(&[c]), Ok(()));
    }

    /// **TASC-10** — many valid calls accepted.
    #[test]
    fn tasc_10_many_valid_accepted() {
        let cs: Vec<ToolArgRecord> = (0..20u8)
            .map(|i| call(i + 1, 1 + (i as usize) % 10, 100 + (i as usize) * 50))
            .collect();
        assert_eq!(validate_schema_compliance(&cs), Ok(()));
    }
}
