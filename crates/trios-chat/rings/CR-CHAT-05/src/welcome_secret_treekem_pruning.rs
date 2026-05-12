//! # CR-CHAT-05 · L-CHAT-5-wst — Welcome-secret / TreeKEM path-pruning defense
//!
//! `[VERIFIED]` Wave-26 lane B — Defends against an adversary that
//! tampers with the TreeKEM `UpdatePath` carried by a `Welcome` so the
//! freshly-added member derives the wrong `joiner_secret`, or accepts
//! a Welcome that prunes private path-secrets the joiner cannot
//! decrypt:
//!
//! * **Empty UpdatePath** — RFC 9420 §12.4.3 mandates at least one
//!   `UpdatePathNode` per non-root copath entry; an empty path is a
//!   downgrade primitive.
//! * **Path length / copath length mismatch** — the path must cover
//!   exactly `copath_resolution_count` levels.
//! * **Pruned encryptions** — every `UpdatePathNode` must have
//!   `node_encryptions.len() == public_key_count` (one HPKE
//!   encryption per resolution leaf). Pruning some entries lets the
//!   adversary force the joiner to derive `joiner_secret` from a
//!   nodes set with mismatched public-key cardinality.
//! * **Group-context epoch splice** — `welcome_epoch !=
//!   tree_hash_epoch` (the `GroupContext` epoch encoded into
//!   `welcome_group_info` must agree with the `tree_hash` epoch).
//! * **Joiner-secret label collision** — `joiner_secret_label` must
//!   be exactly `WST_JOINER_LABEL` (RFC 9420 §8.1, ASCII
//!   `"joiner"`); off-label derivation is a cross-protocol confusion
//!   primitive.
//!
//! See RFC 9420 §12.4.3 (Welcome), §8 (Secret Tree), §7.6.2 (UpdatePath
//! encryption). Five rules are evaluated in fixed order so the error
//! variant is deterministic for any given input.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · WELCOME-TREEKEM-PRUNING`

#![forbid(unsafe_code)]

/// Canonical label for joiner-secret derivation — RFC 9420 §8.1.
pub const WST_JOINER_LABEL: &[u8] = b"joiner";

/// One node along the joiner's direct path. Each node carries one
/// HPKE encryption per public key in its copath resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePathNode {
    /// Number of HPKE public keys in this node's copath resolution.
    pub public_key_count: u32,
    /// HPKE encryptions, one per public key. `len()` must equal
    /// `public_key_count` for the node to be canonical.
    pub node_encryptions: Vec<Vec<u8>>,
}

/// The `UpdatePath` payload carried in the Welcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WelcomeUpdatePath {
    /// One node per non-root level on the joiner's direct path.
    pub nodes: Vec<UpdatePathNode>,
    /// Epoch encoded in the `GroupContext` of `welcome_group_info`.
    pub welcome_epoch: u64,
    /// Epoch of the tree-hash the joiner is being welcomed into.
    pub tree_hash_epoch: u64,
    /// Label used for `joiner_secret` derivation. RFC 9420 mandates
    /// the literal ASCII `"joiner"`.
    pub joiner_secret_label: Vec<u8>,
}

/// Per-group view used to validate the Welcome's `UpdatePath`.
#[derive(Debug, Clone)]
pub struct WelcomeTreeView {
    /// Number of resolution entries the path is expected to cover
    /// (one per non-root copath level).
    pub expected_path_len: u32,
}

/// All ways a Welcome `UpdatePath` can be rejected. Adding variants
/// stays non-breaking via `#[non_exhaustive]`.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WelcomeTreeError {
    /// `nodes.is_empty()` — empty UpdatePath is a downgrade primitive.
    EmptyUpdatePath,
    /// `nodes.len() != expected_path_len` — path / copath length
    /// mismatch.
    PathLengthMismatch,
    /// Some `UpdatePathNode` has `node_encryptions.len() !=
    /// public_key_count` — pruned encryption set.
    PrunedNodeEncryptions,
    /// `welcome_epoch != tree_hash_epoch` — GroupContext epoch
    /// splice.
    GroupContextEpochSplice,
    /// `joiner_secret_label != WST_JOINER_LABEL` — off-label
    /// derivation.
    OffLabelJoinerSecret,
}

/// `[VERIFIED]` Validate a `WelcomeUpdatePath` against the receiving
/// group's view. Returns `Ok(())` iff the path is acceptable.
///
/// The five rules are evaluated in fixed order so the error variant
/// is deterministic for a given input.
pub fn validate_welcome_path(
    path: &WelcomeUpdatePath,
    view: &WelcomeTreeView,
) -> Result<(), WelcomeTreeError> {
    // Rule 1 — non-empty UpdatePath.
    if path.nodes.is_empty() {
        return Err(WelcomeTreeError::EmptyUpdatePath);
    }

    // Rule 2 — path length matches copath resolution length.
    if (path.nodes.len() as u32) != view.expected_path_len {
        return Err(WelcomeTreeError::PathLengthMismatch);
    }

    // Rule 3 — every node has exactly `public_key_count` encryptions.
    for node in &path.nodes {
        if (node.node_encryptions.len() as u32) != node.public_key_count {
            return Err(WelcomeTreeError::PrunedNodeEncryptions);
        }
    }

    // Rule 4 — welcome epoch agrees with tree-hash epoch.
    if path.welcome_epoch != path.tree_hash_epoch {
        return Err(WelcomeTreeError::GroupContextEpochSplice);
    }

    // Rule 5 — joiner-secret label is the canonical `"joiner"`.
    if path.joiner_secret_label.as_slice() != WST_JOINER_LABEL {
        return Err(WelcomeTreeError::OffLabelJoinerSecret);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(n: u32) -> UpdatePathNode {
        UpdatePathNode {
            public_key_count: n,
            node_encryptions: (0..n).map(|i| vec![0xCC, i as u8]).collect(),
        }
    }

    fn canonical_path(levels: usize, epoch: u64) -> WelcomeUpdatePath {
        WelcomeUpdatePath {
            nodes: (0..levels).map(|_| node(2)).collect(),
            welcome_epoch: epoch,
            tree_hash_epoch: epoch,
            joiner_secret_label: WST_JOINER_LABEL.to_vec(),
        }
    }

    // WST-01 — canonical 3-level path accepted.
    #[test]
    fn wst_01_canonical_path_accepted() {
        let p = canonical_path(3, 7);
        let v = WelcomeTreeView {
            expected_path_len: 3,
        };
        assert_eq!(validate_welcome_path(&p, &v), Ok(()));
    }

    // WST-02 — empty UpdatePath rejected.
    #[test]
    fn wst_02_empty_path_rejected() {
        let p = WelcomeUpdatePath {
            nodes: vec![],
            welcome_epoch: 7,
            tree_hash_epoch: 7,
            joiner_secret_label: WST_JOINER_LABEL.to_vec(),
        };
        let v = WelcomeTreeView {
            expected_path_len: 3,
        };
        assert_eq!(
            validate_welcome_path(&p, &v),
            Err(WelcomeTreeError::EmptyUpdatePath)
        );
    }

    // WST-03 — short path (1 < 3) rejected as length mismatch.
    #[test]
    fn wst_03_short_path_rejected() {
        let p = canonical_path(1, 7);
        let v = WelcomeTreeView {
            expected_path_len: 3,
        };
        assert_eq!(
            validate_welcome_path(&p, &v),
            Err(WelcomeTreeError::PathLengthMismatch)
        );
    }

    // WST-04 — overlong path (5 > 3) rejected as length mismatch.
    #[test]
    fn wst_04_overlong_path_rejected() {
        let p = canonical_path(5, 7);
        let v = WelcomeTreeView {
            expected_path_len: 3,
        };
        assert_eq!(
            validate_welcome_path(&p, &v),
            Err(WelcomeTreeError::PathLengthMismatch)
        );
    }

    // WST-05 — pruned encryptions (count mismatch) rejected.
    #[test]
    fn wst_05_pruned_node_encryptions_rejected() {
        let mut p = canonical_path(3, 7);
        // Drop one encryption from the middle node.
        p.nodes[1].node_encryptions.pop();
        let v = WelcomeTreeView {
            expected_path_len: 3,
        };
        assert_eq!(
            validate_welcome_path(&p, &v),
            Err(WelcomeTreeError::PrunedNodeEncryptions)
        );
    }

    // WST-06 — over-stuffed encryptions also rejected.
    #[test]
    fn wst_06_extra_node_encryptions_rejected() {
        let mut p = canonical_path(3, 7);
        p.nodes[0].node_encryptions.push(vec![0xFF; 4]);
        let v = WelcomeTreeView {
            expected_path_len: 3,
        };
        assert_eq!(
            validate_welcome_path(&p, &v),
            Err(WelcomeTreeError::PrunedNodeEncryptions)
        );
    }

    // WST-07 — welcome / tree-hash epoch mismatch rejected.
    #[test]
    fn wst_07_epoch_splice_rejected() {
        let mut p = canonical_path(3, 7);
        p.tree_hash_epoch = 8;
        let v = WelcomeTreeView {
            expected_path_len: 3,
        };
        assert_eq!(
            validate_welcome_path(&p, &v),
            Err(WelcomeTreeError::GroupContextEpochSplice)
        );
    }

    // WST-08 — off-label joiner-secret rejected.
    #[test]
    fn wst_08_off_label_joiner_secret_rejected() {
        let mut p = canonical_path(3, 7);
        p.joiner_secret_label = b"applicant".to_vec();
        let v = WelcomeTreeView {
            expected_path_len: 3,
        };
        assert_eq!(
            validate_welcome_path(&p, &v),
            Err(WelcomeTreeError::OffLabelJoinerSecret)
        );
    }

    // WST-09 — empty joiner-secret label rejected.
    #[test]
    fn wst_09_empty_joiner_label_rejected() {
        let mut p = canonical_path(3, 7);
        p.joiner_secret_label.clear();
        let v = WelcomeTreeView {
            expected_path_len: 3,
        };
        assert_eq!(
            validate_welcome_path(&p, &v),
            Err(WelcomeTreeError::OffLabelJoinerSecret)
        );
    }

    // WST-10 — single-level path with `expected_path_len = 1`
    // accepted (boundary).
    #[test]
    fn wst_10_single_level_path_accepted() {
        let p = canonical_path(1, 0);
        let v = WelcomeTreeView {
            expected_path_len: 1,
        };
        assert_eq!(validate_welcome_path(&p, &v), Ok(()));
    }
}
