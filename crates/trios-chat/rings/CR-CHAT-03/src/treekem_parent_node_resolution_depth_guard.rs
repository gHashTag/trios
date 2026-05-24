//! # CR-CHAT-03 — TreeKEM parent node resolution depth guard (Wave-106 Lane A)
//!
//! RATCHET TREE — parent node resolution must not skip too many levels.
//!
//! In TreeKEM, when a leaf is blanked (member removed), the parent
//! nodes above it must be resolved by finding the nearest non-blank
//! descendant. If resolution skips too many levels:
//!
//! * **Tree depth attack** — an adversary blanks many leaves, forcing
//!   resolution to traverse the entire tree height, causing O(n)
//!   computation instead of O(log n).
//! * **Path hijacking** — a deeply-resolved node can be influenced by
//!   a distant leaf, allowing an attacker to inject their public key
//!   into the path of an unrelated member.
//! * **State explosion** — unbounded resolution depth causes the
//!   resolution vector to grow unboundedly, consuming memory.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Resolution depth <= `TPNR_MAX_DEPTH`.
//! 2. Node index must be > 0.
//! 3. Resolution path must be strictly increasing.
//! 4. No duplicate nodes in resolution path.
//! 5. Total resolutions <= `TPNR_MAX_RESOLUTIONS`.
//! 6. Node index must be < `TPNR_MAX_NODES`.
//!
//! Tests **TPNR-01..10**. Error enum [`ResolutionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RESOLUTION-DEPTH`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum resolution depth.
pub const TPNR_MAX_DEPTH: u32 = 32;

/// Maximum nodes in tree.
pub const TPNR_MAX_NODES: u64 = 1_000_000;

/// Maximum resolutions per batch.
pub const TPNR_MAX_RESOLUTIONS: usize = 1024;

/// A node resolution record.
#[derive(Debug, Clone)]
pub struct NodeResolution {
    /// Index of the node being resolved.
    pub node_index: u64,
    /// Depth of resolution (levels traversed).
    pub depth: u32,
    /// Path of node indices traversed during resolution.
    pub path: Vec<u64>,
}

/// All ways node resolution validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResolutionError {
    /// Depth exceeded.
    DepthExceeded { idx: usize, depth: u32, max: u32 },
    /// Zero node index.
    ZeroNode(usize),
    /// Path not strictly increasing.
    NotIncreasing { idx: usize, pos: usize, prev: u64, current: u64 },
    /// Duplicate node in path.
    DuplicateInPath { idx: usize, node: u64 },
    /// Too many resolutions.
    TooMany { got: usize, max: usize },
    /// Node index exceeds maximum.
    NodeExceedsMax { idx: usize, node: u64, max: u64 },
}

/// `[VERIFIED]` Validate TreeKEM parent node resolution depth.
pub fn validate_node_resolutions(
    resolutions: &[NodeResolution],
) -> Result<(), ResolutionError> {
    if resolutions.len() > TPNR_MAX_RESOLUTIONS {
        return Err(ResolutionError::TooMany {
            got: resolutions.len(),
            max: TPNR_MAX_RESOLUTIONS,
        });
    }
    for (i, r) in resolutions.iter().enumerate() {
        if r.node_index == 0 {
            return Err(ResolutionError::ZeroNode(i));
        }
        if r.node_index >= TPNR_MAX_NODES {
            return Err(ResolutionError::NodeExceedsMax {
                idx: i,
                node: r.node_index,
                max: TPNR_MAX_NODES,
            });
        }
        if r.depth > TPNR_MAX_DEPTH {
            return Err(ResolutionError::DepthExceeded {
                idx: i,
                depth: r.depth,
                max: TPNR_MAX_DEPTH,
            });
        }
        let mut seen: BTreeSet<u64> = BTreeSet::new();
        for (pos, &node) in r.path.iter().enumerate() {
            if !seen.insert(node) {
                return Err(ResolutionError::DuplicateInPath { idx: i, node });
            }
            if pos > 0 && node <= r.path[pos - 1] {
                return Err(ResolutionError::NotIncreasing {
                    idx: i,
                    pos,
                    prev: r.path[pos - 1],
                    current: node,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolution(node: u64, depth: u32, path: Vec<u64>) -> NodeResolution {
        NodeResolution { node_index: node, depth, path }
    }

    fn valid_resolutions() -> Vec<NodeResolution> {
        vec![
            resolution(1, 2, vec![1, 3, 7]),
            resolution(2, 1, vec![2, 5]),
        ]
    }

    /// **TPNR-01** — depth exceeded rejected.
    #[test]
    fn tpnr_01_depth_exceeded_rejected() {
        let r = resolution(1, TPNR_MAX_DEPTH + 1, vec![1]);
        assert_eq!(
            validate_node_resolutions(&[r]),
            Err(ResolutionError::DepthExceeded {
                idx: 0,
                depth: TPNR_MAX_DEPTH + 1,
                max: TPNR_MAX_DEPTH,
            })
        );
    }

    /// **TPNR-02** — zero node rejected.
    #[test]
    fn tpnr_02_zero_node_rejected() {
        let r = NodeResolution { node_index: 0, depth: 1, path: vec![1] };
        assert_eq!(
            validate_node_resolutions(&[r]),
            Err(ResolutionError::ZeroNode(0))
        );
    }

    /// **TPNR-03** — not increasing rejected.
    #[test]
    fn tpnr_03_not_increasing_rejected() {
        let r = resolution(1, 2, vec![10, 5, 15]);
        assert_eq!(
            validate_node_resolutions(&[r]),
            Err(ResolutionError::NotIncreasing {
                idx: 0,
                pos: 1,
                prev: 10,
                current: 5,
            })
        );
    }

    /// **TPNR-04** — duplicate in path rejected.
    #[test]
    fn tpnr_04_duplicate_rejected() {
        let r = resolution(1, 2, vec![5, 10, 5]);
        assert_eq!(
            validate_node_resolutions(&[r]),
            Err(ResolutionError::DuplicateInPath { idx: 0, node: 5 })
        );
    }

    /// **TPNR-05** — too many rejected.
    #[test]
    fn tpnr_05_too_many_rejected() {
        let rs: Vec<NodeResolution> = (0..=TPNR_MAX_RESOLUTIONS)
            .map(|i| NodeResolution {
                node_index: (i as u64) + 1,
                depth: 1,
                path: vec![(i as u64) + 1],
            })
            .collect();
        assert_eq!(
            validate_node_resolutions(&rs),
            Err(ResolutionError::TooMany {
                got: TPNR_MAX_RESOLUTIONS + 1,
                max: TPNR_MAX_RESOLUTIONS,
            })
        );
    }

    /// **TPNR-06** — node exceeds max rejected.
    #[test]
    fn tpnr_06_node_exceeds_max_rejected() {
        let r = resolution(TPNR_MAX_NODES, 1, vec![TPNR_MAX_NODES]);
        assert_eq!(
            validate_node_resolutions(&[r]),
            Err(ResolutionError::NodeExceedsMax {
                idx: 0,
                node: TPNR_MAX_NODES,
                max: TPNR_MAX_NODES,
            })
        );
    }

    /// **TPNR-07** — valid accepted.
    #[test]
    fn tpnr_07_valid_accepted() {
        assert_eq!(validate_node_resolutions(&valid_resolutions()), Ok(()));
    }

    /// **TPNR-08** — empty accepted.
    #[test]
    fn tpnr_08_empty_accepted() {
        assert_eq!(validate_node_resolutions(&[]), Ok(()));
    }

    /// **TPNR-09** — single node accepted.
    #[test]
    fn tpnr_09_single_accepted() {
        let r = resolution(1, 0, vec![1]);
        assert_eq!(validate_node_resolutions(&[r]), Ok(()));
    }

    /// **TPNR-10** — max depth boundary accepted.
    #[test]
    fn tpnr_10_max_depth_accepted() {
        let path: Vec<u64> = (0..TPNR_MAX_DEPTH as usize).map(|i| (i as u64) + 1).collect();
        let r = resolution(1, TPNR_MAX_DEPTH, path);
        assert_eq!(validate_node_resolutions(&[r]), Ok(()));
    }
}
