//! # CR-CHAT-06 — Tool call chain depth guard (Wave-61 Lane B)
//!
//! AGENT SAFETY — prevent deep tool call chains, R-CHAT-7.
//!
//! An LLM can be tricked into an infinite or very deep chain of tool
//! calls: tool A output triggers tool B, whose output triggers tool C,
//! etc. This can:
//!
//! * **Exhaust compute** — unbounded CPU/memory via recursive calls.
//! * **Escalate privileges** — each call slightly widens scope until
//!   a dangerous operation is reached.
//! * **Leak data** — chain output from internal tool to external API.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Chain depth <= `TCCD_MAX_DEPTH`.
//! 2. No circular tool calls (A -> B -> A).
//! 3. Each chain step has non-empty input.
//! 4. No duplicate tool in same chain.
//! 5. Total chain input size <= `TCCD_MAX_TOTAL_INPUT`.
//! 6. Chain must terminate (has a final non-tool output).
//!
//! Tests **TCCD-01..10**. Error enum [`ToolChainError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TOOL-CHAIN-DEPTH`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum chain depth.
pub const TCCD_MAX_DEPTH: usize = 8;

/// Maximum total input size across chain.
pub const TCCD_MAX_TOTAL_INPUT: usize = 65536;

/// All ways tool chain validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolChainError {
    /// Chain too deep.
    ChainTooDeep,
    /// Circular call detected.
    CircularCall,
    /// Empty input at step.
    EmptyInput,
    /// Duplicate tool in chain.
    DuplicateTool,
    /// Total input too large.
    TotalInputTooLarge,
    /// Chain does not terminate.
    NonTerminating,
}

/// A step in the tool call chain.
#[derive(Debug, Clone)]
pub struct ChainStep {
    /// Tool identifier.
    pub tool_id: u8,
    /// Input size in bytes.
    pub input_size: usize,
    /// Whether this step produces another tool call.
    pub produces_tool_call: bool,
}

/// `[VERIFIED]` Validate a tool call chain.
pub fn validate_tool_chain(steps: &[ChainStep]) -> Result<(), ToolChainError> {
    if steps.is_empty() {
        return Ok(());
    }
    if steps.len() > TCCD_MAX_DEPTH {
        return Err(ToolChainError::ChainTooDeep);
    }
    let total_input: usize = steps.iter().map(|s| s.input_size).sum();
    if total_input > TCCD_MAX_TOTAL_INPUT {
        return Err(ToolChainError::TotalInputTooLarge);
    }
    let mut seen = BTreeSet::new();
    for (i, step) in steps.iter().enumerate() {
        if step.input_size == 0 {
            return Err(ToolChainError::EmptyInput);
        }
        if i > 0 && step.tool_id == steps[0].tool_id {
            return Err(ToolChainError::CircularCall);
        }
        if !seen.insert(step.tool_id) {
            return Err(ToolChainError::DuplicateTool);
        }
    }
    if steps.last().map(|s| s.produces_tool_call).unwrap_or(false) {
        return Err(ToolChainError::NonTerminating);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(tool: u8, size: usize, produces: bool) -> ChainStep {
        ChainStep { tool_id: tool, input_size: size, produces_tool_call: produces }
    }

    fn good_chain() -> Vec<ChainStep> {
        vec![step(1, 100, true), step(2, 200, true), step(3, 50, false)]
    }

    /// **TCCD-01** — chain too deep rejected.
    #[test]
    fn tccd_01_too_deep_rejected() {
        let chain: Vec<ChainStep> = (0..=TCCD_MAX_DEPTH)
            .map(|i| step(i as u8, 10, i < TCCD_MAX_DEPTH))
            .collect();
        assert_eq!(
            validate_tool_chain(&chain),
            Err(ToolChainError::ChainTooDeep)
        );
    }

    /// **TCCD-02** — circular call rejected.
    #[test]
    fn tccd_02_circular_rejected() {
        let chain = vec![step(1, 10, true), step(2, 10, true), step(1, 10, false)];
        assert_eq!(
            validate_tool_chain(&chain),
            Err(ToolChainError::CircularCall)
        );
    }

    /// **TCCD-03** — empty input rejected.
    #[test]
    fn tccd_03_empty_input_rejected() {
        let chain = vec![step(1, 0, false)];
        assert_eq!(
            validate_tool_chain(&chain),
            Err(ToolChainError::EmptyInput)
        );
    }

    /// **TCCD-04** — duplicate tool rejected.
    #[test]
    fn tccd_04_duplicate_rejected() {
        let chain = vec![step(1, 10, true), step(2, 10, true), step(2, 10, false)];
        assert_eq!(
            validate_tool_chain(&chain),
            Err(ToolChainError::DuplicateTool)
        );
    }

    /// **TCCD-05** — total input too large rejected.
    #[test]
    fn tccd_05_total_input_rejected() {
        let chain = vec![step(1, TCCD_MAX_TOTAL_INPUT + 1, false)];
        assert_eq!(
            validate_tool_chain(&chain),
            Err(ToolChainError::TotalInputTooLarge)
        );
    }

    /// **TCCD-06** — non-terminating rejected.
    #[test]
    fn tccd_06_non_terminating_rejected() {
        let chain = vec![step(1, 10, true), step(2, 10, true)];
        assert_eq!(
            validate_tool_chain(&chain),
            Err(ToolChainError::NonTerminating)
        );
    }

    /// **TCCD-07** — good chain accepted.
    #[test]
    fn tccd_07_good_accepted() {
        assert_eq!(validate_tool_chain(&good_chain()), Ok(()));
    }

    /// **TCCD-08** — single terminating step accepted.
    #[test]
    fn tccd_08_single_accepted() {
        assert_eq!(validate_tool_chain(&[step(1, 100, false)]), Ok(()));
    }

    /// **TCCD-09** — empty chain accepted.
    #[test]
    fn tccd_09_empty_accepted() {
        assert_eq!(validate_tool_chain(&[]), Ok(()));
    }

    /// **TCCD-10** — max depth boundary accepted.
    #[test]
    fn tccd_10_max_depth_accepted() {
        let chain: Vec<ChainStep> = (0..TCCD_MAX_DEPTH)
            .map(|i| step(i as u8, 10, i < TCCD_MAX_DEPTH - 1))
            .collect();
        assert_eq!(validate_tool_chain(&chain), Ok(()));
    }
}
