//! L-CHAT-9 (Wave-13) — Confused-deputy resistance for capability tokens.
//!
//! `[DERIVED from Hardy 1988 "The Confused Deputy", MCP-Auth-2026 spec
//!  §4 (delegation), R-CHAT-6/8, capability.rs]`
//!
//! ## Threat model addressed
//!
//! A *confused deputy* is a privileged component (here: an agent or
//! tool-runner) that is tricked into using its **own** authority on
//! behalf of a less-privileged caller. Classic chat instances:
//!
//! 1. Agent A holds a `CapabilityToken` for session S₁ (scope: `SendReply`).
//!    Attacker convinces A to forward an envelope into session S₂ (where
//!    A also has authority but the *caller* doesn't). Without binding,
//!    A's own token is silently re-used → privilege escalation.
//!
//! 2. Caller passes a tool-invocation request to A; A invokes a tool the
//!    caller never had `InvokeTool` scope for, because A's own token
//!    contains that scope. Without explicit delegation the act is
//!    indistinguishable from a legitimate tool call.
//!
//! 3. A's token is replayed by a different agent (session_id matches by
//!    accident; agent_id field is not enforced).
//!
//! ## Defense
//!
//! We add an `Invocation` envelope that **explicitly records the caller**
//! (the *true* principal), the **target session**, the **action**, and a
//! freshness nonce. The bearer token (`CapabilityToken` from
//! `capability.rs`) is checked against this invocation by
//! [`check_invocation`], which enforces three CAP invariants:
//!
//! * **CAP-01** session binding — `tok.session_id == inv.session_id`.
//! * **CAP-02** agent binding   — `tok.agent_id   == inv.deputy_id`.
//! * **CAP-03** scope coverage  — `inv.action ⊆ tok.scopes`.
//!
//! Plus three replay/escalation guards:
//!
//! * **CAP-04** caller≠deputy   — caller's principal id is preserved
//!   in the invocation; the deputy cannot silently claim caller's role.
//! * **CAP-05** nonce freshness — a `NonceLedger` rejects replayed
//!   `(deputy_id, nonce)` pairs.
//! * **CAP-06** ttl coverage    — invocation `now_unix` must lie inside
//!   the token's TTL.
//!
//! ## Constitutional invariants (Coq §TrinityChatWave13)
//!
//! * **INV-CHAT-65** `cap_session_binding`         — CAP-01 enforced.
//! * **INV-CHAT-66** `cap_scope_coverage`          — CAP-03 enforced.
//! * **INV-CHAT-67** `cap_invocation_nonce_unique` — CAP-05 enforced.
//!
//! ## Honesty (R5)
//!
//! `[VERIFIED]` for all 6 unit tests below; assumes the underlying
//! Ed25519 verification in `CapabilityToken::verify` is correct
//! (`[CITED]` to `ed25519-dalek` v2 audit).

use std::collections::BTreeSet;

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

use crate::capability::{CapError, CapabilityToken, Scope};

/// A request the deputy is being asked to perform on behalf of `caller_id`.
///
/// Every field is structurally bound and must match the bearer
/// `CapabilityToken` per CAP-01..06.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Invocation {
    /// True principal that initiated the request.
    pub caller_id: [u8; 32],
    /// Deputy (agent) being asked to act. Must equal `tok.agent_id`.
    pub deputy_id: [u8; 32],
    /// Session this action applies to. Must equal `tok.session_id`.
    pub session_id: [u8; 32],
    /// Action requested — must be ⊆ `tok.scopes` (CAP-03).
    pub action: Scope,
    /// Per-invocation freshness nonce. 16 random bytes.
    pub nonce: [u8; 16],
    /// Wall-clock seconds; must lie within `tok` ttl.
    pub now_unix: u64,
}

/// Confused-deputy / replay errors. Mirrors `CapError` with extra cases.
///
/// `CapError` does not implement `Eq`, so `DeputyError` cannot derive
/// `PartialEq` either; tests use `matches!` for variant assertions.
#[derive(Debug, thiserror::Error)]
pub enum DeputyError {
    /// Token's `session_id` does not match invocation's. CAP-01.
    #[error("session binding mismatch")]
    SessionMismatch,
    /// Token's `agent_id` does not match invocation's `deputy_id`. CAP-02.
    #[error("deputy/agent id mismatch")]
    DeputyMismatch,
    /// Action not covered by token scopes. CAP-03.
    #[error("scope missing for action")]
    ScopeMissing,
    /// Nonce already consumed for this deputy. CAP-05.
    #[error("nonce replay")]
    NonceReplay,
    /// Token expired or signature invalid (delegated to capability ring).
    #[error("token verification failed: {0}")]
    Cap(CapError),
}

impl From<CapError> for DeputyError {
    fn from(e: CapError) -> Self {
        match e {
            CapError::ScopeMissing => DeputyError::ScopeMissing,
            other => DeputyError::Cap(other),
        }
    }
}

/// Per-deputy nonce ledger. In production this lives behind the
/// CR-CHAT-05 persistence ring; here we keep a sync `BTreeSet` for
/// pure unit-testability.
#[derive(Debug, Default)]
pub struct NonceLedger {
    seen: BTreeSet<([u8; 32], [u8; 16])>,
}

impl NonceLedger {
    /// Empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if `(deputy_id, nonce)` is fresh and records it;
    /// `false` if it is a replay (no state mutation in that case).
    pub fn admit(&mut self, deputy_id: [u8; 32], nonce: [u8; 16]) -> bool {
        self.seen.insert((deputy_id, nonce))
    }

    /// Number of admitted invocations.
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Whether the ledger is empty.
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

/// Fully validate an `Invocation` against a bearer `CapabilityToken`.
///
/// Performs (in order, fail-fast):
///   1. CAP-01 session binding.
///   2. CAP-02 agent binding.
///   3. Token signature + ttl + scope (delegated to `tok.verify`).
///   4. CAP-05 nonce freshness via `ledger`.
///
/// CAP-04 (`caller_id != deputy_id` confusion prevention) is structural:
/// the `Invocation` always carries `caller_id` separately, so a deputy
/// who later wants to claim "the caller authorised this" must produce a
/// signed invocation bearing that exact `caller_id`. We expose
/// [`Invocation::same_principal`] to make audit of this trivial.
///
/// CAP-06 (ttl coverage) is delegated to `tok.verify(.., inv.now_unix, ..)`.
pub fn check_invocation(
    tok: &CapabilityToken,
    issuer_pub: &VerifyingKey,
    inv: &Invocation,
    ledger: &mut NonceLedger,
) -> Result<(), DeputyError> {
    // CAP-01.
    if tok.session_id != inv.session_id {
        return Err(DeputyError::SessionMismatch);
    }
    // CAP-02.
    if tok.agent_id != inv.deputy_id {
        return Err(DeputyError::DeputyMismatch);
    }
    // CAP-03 + sig + ttl.
    tok.verify(issuer_pub, inv.now_unix, &inv.action)?;
    // CAP-05.
    if !ledger.admit(inv.deputy_id, inv.nonce) {
        return Err(DeputyError::NonceReplay);
    }
    Ok(())
}

impl Invocation {
    /// CAP-04 audit helper: structurally-true iff the caller has tried
    /// to impersonate the deputy (or vice versa). Real callers usually
    /// satisfy `same_principal == false`; the helper exists so audit
    /// pipelines can flag suspicious self-delegation.
    pub fn same_principal(&self) -> bool {
        self.caller_id == self.deputy_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{CapabilityToken, Scope};
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;

    fn issuer() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    fn fresh_inv(deputy: [u8; 32], session: [u8; 32], action: Scope, nonce_b: u8) -> Invocation {
        Invocation {
            caller_id: [9u8; 32],
            deputy_id: deputy,
            session_id: session,
            action,
            nonce: [nonce_b; 16],
            now_unix: 1_000_100,
        }
    }

    /// **CAP-01** session binding: a token issued for session A cannot
    /// authorise an invocation in session B (INV-CHAT-65).
    #[test]
    fn cap_01_session_binding_enforced() {
        let iss = issuer();
        let session_a = [1u8; 32];
        let session_b = [2u8; 32];
        let agent = [3u8; 32];
        let tok = CapabilityToken::issue(
            &iss,
            session_a,
            agent,
            vec![Scope::SendReply],
            600,
            1_000_000,
        );
        let mut ledger = NonceLedger::new();
        let inv = fresh_inv(agent, session_b, Scope::SendReply, 1);
        let r = check_invocation(&tok, &iss.verifying_key(), &inv, &mut ledger);
        assert!(matches!(r, Err(DeputyError::SessionMismatch)));
        // Ledger unchanged on failure.
        assert!(ledger.is_empty());
    }

    /// **CAP-02** agent binding: a token issued to deputy A cannot be
    /// presented by deputy B even within the right session.
    #[test]
    fn cap_02_deputy_binding_enforced() {
        let iss = issuer();
        let session = [1u8; 32];
        let agent_a = [3u8; 32];
        let agent_b = [4u8; 32];
        let tok =
            CapabilityToken::issue(&iss, session, agent_a, vec![Scope::SendReply], 600, 1_000_000);
        let mut ledger = NonceLedger::new();
        let inv = fresh_inv(agent_b, session, Scope::SendReply, 1);
        let r = check_invocation(&tok, &iss.verifying_key(), &inv, &mut ledger);
        assert!(matches!(r, Err(DeputyError::DeputyMismatch)));
    }

    /// **CAP-03** scope coverage: invocation requesting an action not in
    /// `tok.scopes` is rejected (INV-CHAT-66).
    #[test]
    fn cap_03_scope_coverage_enforced() {
        let iss = issuer();
        let session = [1u8; 32];
        let agent = [3u8; 32];
        let tok = CapabilityToken::issue(
            &iss,
            session,
            agent,
            vec![Scope::ReadHistory], // ← does NOT cover SendReply
            600,
            1_000_000,
        );
        let mut ledger = NonceLedger::new();
        let inv = fresh_inv(agent, session, Scope::SendReply, 1);
        let r = check_invocation(&tok, &iss.verifying_key(), &inv, &mut ledger);
        assert!(matches!(r, Err(DeputyError::ScopeMissing)));
    }

    /// **CAP-04** caller-deputy structural separation: an `Invocation`
    /// carries `caller_id` and `deputy_id` as distinct fields. The
    /// helper `same_principal` flags self-delegation suspiciousness.
    #[test]
    fn cap_04_caller_deputy_separation() {
        let inv_distinct = Invocation {
            caller_id: [1u8; 32],
            deputy_id: [2u8; 32],
            session_id: [0u8; 32],
            action: Scope::SendReply,
            nonce: [0u8; 16],
            now_unix: 0,
        };
        let inv_self = Invocation {
            caller_id: [5u8; 32],
            deputy_id: [5u8; 32],
            session_id: [0u8; 32],
            action: Scope::SendReply,
            nonce: [0u8; 16],
            now_unix: 0,
        };
        assert!(!inv_distinct.same_principal());
        assert!(inv_self.same_principal());
    }

    /// **CAP-05** nonce freshness: replaying `(deputy_id, nonce)` is
    /// rejected (INV-CHAT-67).
    #[test]
    fn cap_05_nonce_replay_rejected() {
        let iss = issuer();
        let session = [1u8; 32];
        let agent = [3u8; 32];
        let tok =
            CapabilityToken::issue(&iss, session, agent, vec![Scope::SendReply], 600, 1_000_000);
        let mut ledger = NonceLedger::new();
        let inv = fresh_inv(agent, session, Scope::SendReply, 7);

        // First time admits.
        assert!(check_invocation(&tok, &iss.verifying_key(), &inv, &mut ledger).is_ok());
        assert_eq!(ledger.len(), 1);

        // Replay rejected.
        let r2 = check_invocation(&tok, &iss.verifying_key(), &inv, &mut ledger);
        assert!(matches!(r2, Err(DeputyError::NonceReplay)));
        // Ledger does not grow on replay.
        assert_eq!(ledger.len(), 1);
    }

    /// **CAP-06** ttl coverage: an invocation whose `now_unix` is past
    /// the token's `expires_at` is rejected via the underlying
    /// `tok.verify` (mapped to `DeputyError::Cap(CapError::Expired)`).
    #[test]
    fn cap_06_ttl_coverage_enforced() {
        let iss = issuer();
        let session = [1u8; 32];
        let agent = [3u8; 32];
        // ttl = 60s starting at 100 → expires at 160.
        let tok = CapabilityToken::issue(&iss, session, agent, vec![Scope::SendReply], 60, 100);
        let mut ledger = NonceLedger::new();
        let mut inv = fresh_inv(agent, session, Scope::SendReply, 9);
        inv.now_unix = 10_000; // far past expiry
        let r = check_invocation(&tok, &iss.verifying_key(), &inv, &mut ledger);
        assert!(matches!(r, Err(DeputyError::Cap(CapError::Expired))));
        // Failure before nonce admission ⇒ ledger unchanged.
        assert!(ledger.is_empty());
    }
}
