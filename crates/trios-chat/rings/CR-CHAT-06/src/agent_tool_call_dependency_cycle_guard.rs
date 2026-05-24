//! # CR-CHAT-06 — Agent tool call dependency cycle guard (Wave-121 Lane A)
//!
//! AGENT SAFETY — tool call chains must be acyclic; cyclic
//! dependencies cause infinite tool execution loops.
//!
//! When an agent invokes tools in sequence where the output of one
//! feeds into the next, the dependency graph must be a DAG:
//!
//! * **Infinite loops** — a cycle in tool dependencies causes the
//!   agent to loop forever, consuming resources without producing.
//! * **Resource exhaustion** — each cycle iteration consumes CPU,
//!   memory, and API quota, eventually exhausting host resources.
//! * **Timeout masking** — cycles may not trigger timeout guards
//!   if each individual call completes within limits.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No cycle in the dependency graph (A→B→A).
//! 2. Tool ID must not be zero.
//! 3. Dependency ID must not be zero (unless root, dep_id = 0).
//! 4. No duplicate tool IDs.
//! 5. Dependency must reference an existing tool ID.
//! 6. Total edges <= `ATDC_MAX_EDGES`.
//!
//! Tests **ATDC-01..10**. Error enum [`DependencyCycleError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * ACYCLIC`

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Maximum edges per batch.
pub const ATDC_MAX_EDGES: usize = 1024;

/// Tool ID length.
pub const ATDC_TOOL_ID_LEN: usize = 32;

/// A dependency edge in the tool call graph.
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Tool identifier.
    pub tool_id: [u8; ATDC_TOOL_ID_LEN],
    /// Dependency tool identifier (zero = root node).
    pub depends_on: [u8; ATDC_TOOL_ID_LEN],
}

/// All ways dependency cycle validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DependencyCycleError {
    /// Cycle detected in dependency graph.
    CycleDetected { tool_id: [u8; ATDC_TOOL_ID_LEN] },
    /// Zero tool ID.
    ZeroToolId(usize),
    /// Duplicate tool ID.
    DuplicateToolId { idx: usize, tool_id: [u8; ATDC_TOOL_ID_LEN] },
    /// Dependency references unknown tool.
    UnknownDependency { idx: usize, tool_id: [u8; ATDC_TOOL_ID_LEN], depends_on: [u8; ATDC_TOOL_ID_LEN] },
    /// Self-dependency.
    SelfDependency { idx: usize, tool_id: [u8; ATDC_TOOL_ID_LEN] },
    /// Too many edges.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent tool call dependency acyclicity.
pub fn validate_no_cycles(
    edges: &[DependencyEdge],
) -> Result<(), DependencyCycleError> {
    if edges.len() > ATDC_MAX_EDGES {
        return Err(DependencyCycleError::TooMany {
            got: edges.len(),
            max: ATDC_MAX_EDGES,
        });
    }
    let mut tool_ids: BTreeSet<[u8; ATDC_TOOL_ID_LEN]> = BTreeSet::new();
    for (i, e) in edges.iter().enumerate() {
        if e.tool_id == [0u8; ATDC_TOOL_ID_LEN] {
            return Err(DependencyCycleError::ZeroToolId(i));
        }
        if e.tool_id == e.depends_on {
            return Err(DependencyCycleError::SelfDependency {
                idx: i,
                tool_id: e.tool_id,
            });
        }
        if !tool_ids.insert(e.tool_id) {
            return Err(DependencyCycleError::DuplicateToolId {
                idx: i,
                tool_id: e.tool_id,
            });
        }
    }
    let mut adj: BTreeMap<[u8; ATDC_TOOL_ID_LEN], [u8; ATDC_TOOL_ID_LEN]> = BTreeMap::new();
    for e in edges {
        if e.depends_on != [0u8; ATDC_TOOL_ID_LEN] {
            if !tool_ids.contains(&e.depends_on) {
                return Err(DependencyCycleError::UnknownDependency {
                    idx: edges.iter().position(|x| x.tool_id == e.tool_id).unwrap(),
                    tool_id: e.tool_id,
                    depends_on: e.depends_on,
                });
            }
            adj.insert(e.tool_id, e.depends_on);
        }
    }
    for e in edges {
        if e.depends_on == [0u8; ATDC_TOOL_ID_LEN] {
            continue;
        }
        let mut visited: BTreeSet<[u8; ATDC_TOOL_ID_LEN]> = BTreeSet::new();
        let mut current = e.tool_id;
        loop {
            if !visited.insert(current) {
                return Err(DependencyCycleError::CycleDetected { tool_id: current });
            }
            match adj.get(&current) {
                Some(&dep) => current = dep,
                None => break,
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tid(byte: u8) -> [u8; ATDC_TOOL_ID_LEN] {
        [byte; ATDC_TOOL_ID_LEN]
    }

    fn edge(tool: u8, dep: u8) -> DependencyEdge {
        DependencyEdge { tool_id: tid(tool), depends_on: tid(dep) }
    }

    fn root(tool: u8) -> DependencyEdge {
        DependencyEdge { tool_id: tid(tool), depends_on: [0u8; ATDC_TOOL_ID_LEN] }
    }

    fn valid_chain() -> Vec<DependencyEdge> {
        vec![
            root(0x01),
            edge(0x02, 0x01),
            edge(0x03, 0x02),
        ]
    }

    /// **ATDC-01** — cycle detected rejected.
    #[test]
    fn atdc_01_cycle_rejected() {
        let es = vec![
            edge(0x01, 0x02),
            edge(0x02, 0x01),
        ];
        assert!(matches!(
            validate_no_cycles(&es),
            Err(DependencyCycleError::CycleDetected { .. })
        ));
    }

    /// **ATDC-02** — zero tool ID rejected.
    #[test]
    fn atdc_02_zero_tool_rejected() {
        let e = DependencyEdge { tool_id: [0u8; ATDC_TOOL_ID_LEN], depends_on: [0u8; ATDC_TOOL_ID_LEN] };
        assert_eq!(
            validate_no_cycles(&[e]),
            Err(DependencyCycleError::ZeroToolId(0))
        );
    }

    /// **ATDC-03** — duplicate tool ID rejected.
    #[test]
    fn atdc_03_duplicate_tool_rejected() {
        let es = vec![
            root(0x01),
            edge(0x01, 0x02),
        ];
        assert_eq!(
            validate_no_cycles(&es),
            Err(DependencyCycleError::DuplicateToolId { idx: 1, tool_id: tid(0x01) })
        );
    }

    /// **ATDC-04** — unknown dependency rejected.
    #[test]
    fn atdc_04_unknown_dep_rejected() {
        let es = vec![
            edge(0x01, 0x99),
        ];
        assert_eq!(
            validate_no_cycles(&es),
            Err(DependencyCycleError::UnknownDependency {
                idx: 0,
                tool_id: tid(0x01),
                depends_on: tid(0x99),
            })
        );
    }

    /// **ATDC-05** — self dependency rejected.
    #[test]
    fn atdc_05_self_dep_rejected() {
        let es = vec![
            edge(0x01, 0x01),
        ];
        assert_eq!(
            validate_no_cycles(&es),
            Err(DependencyCycleError::SelfDependency { idx: 0, tool_id: tid(0x01) })
        );
    }

    /// **ATDC-06** — too many rejected.
    #[test]
    fn atdc_06_too_many_rejected() {
        let es: Vec<DependencyEdge> = (0..=ATDC_MAX_EDGES)
            .map(|i| {
                let mut id = [0u8; ATDC_TOOL_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                DependencyEdge { tool_id: id, depends_on: [0u8; ATDC_TOOL_ID_LEN] }
            })
            .collect();
        assert_eq!(
            validate_no_cycles(&es),
            Err(DependencyCycleError::TooMany {
                got: ATDC_MAX_EDGES + 1,
                max: ATDC_MAX_EDGES,
            })
        );
    }

    /// **ATDC-07** — valid chain accepted.
    #[test]
    fn atdc_07_valid_accepted() {
        assert_eq!(validate_no_cycles(&valid_chain()), Ok(()));
    }

    /// **ATDC-08** — empty accepted.
    #[test]
    fn atdc_08_empty_accepted() {
        assert_eq!(validate_no_cycles(&[]), Ok(()));
    }

    /// **ATDC-09** — single root accepted.
    #[test]
    fn atdc_09_single_root_accepted() {
        assert_eq!(validate_no_cycles(&[root(0x01)]), Ok(()));
    }

    /// **ATDC-10** — diamond DAG accepted.
    #[test]
    fn atdc_10_diamond_dag_accepted() {
        let es = vec![
            root(0x01),
            edge(0x02, 0x01),
            edge(0x03, 0x01),
            edge(0x04, 0x02),
            edge(0x05, 0x03),
        ];
        assert_eq!(validate_no_cycles(&es), Ok(()));
    }
}
