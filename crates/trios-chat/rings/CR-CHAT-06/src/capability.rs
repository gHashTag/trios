//! L-CHAT-6: capability tokens + signed tool manifest verifier.
//!
//! `[DERIVED from MCP-Auth-2026 + A2A spec, design §3.6, R-CHAT-6/8]`
//!
//! Constitutional invariants:
//! - **INV-CHAT-2** `agent_capability_bound` — `agent action set ⊆ capability.scope`
//! - **R-CHAT-6** TOOLS ARE SIGNED PROMPTS — every tool manifest carries Ed25519 sig
//! - **R-CHAT-8** SESSION-SCOPED CAPABILITY — token bound to (session_id, ttl)

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Single capability scope item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    /// Read message history within session.
    ReadHistory,
    /// Send a chat reply.
    SendReply,
    /// Invoke a registered tool by name.
    InvokeTool(String),
    /// Fetch a URL on a domain allow-list.
    FetchUrl(String),
}

/// Session-scoped capability token. `[DERIVED]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    /// Session this token applies to.
    pub session_id: [u8; 32],
    /// Bearer agent identity.
    pub agent_id: [u8; 32],
    /// Allowed scopes.
    pub scopes: Vec<Scope>,
    /// UNIX seconds; verified ttl < 3600.
    pub expires_at: u64,
    /// 16-byte fresh nonce per token.
    pub nonce: [u8; 16],
    /// Ed25519 signature by Issuer over canonical bytes.
    pub sig: Vec<u8>,
}

impl CapabilityToken {
    /// Canonical bytes for signing/verification. `[VERIFIED via test]`
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(128);
        buf.extend_from_slice(&self.session_id);
        buf.extend_from_slice(&self.agent_id);
        let scopes_json = serde_json::to_vec(&self.scopes).expect("scopes serialize");
        buf.extend_from_slice(&(scopes_json.len() as u32).to_le_bytes());
        buf.extend_from_slice(&scopes_json);
        buf.extend_from_slice(&self.expires_at.to_le_bytes());
        buf.extend_from_slice(&self.nonce);
        buf
    }

    /// Issue a signed token. `[VERIFIED]`
    ///
    /// Panics if `ttl_secs > 3600` — this is the INV-CHAT-2 hard ceiling.
    pub fn issue(
        issuer: &SigningKey,
        session_id: [u8; 32],
        agent_id: [u8; 32],
        scopes: Vec<Scope>,
        ttl_secs: u64,
        now_unix: u64,
    ) -> Self {
        assert!(ttl_secs <= 3600, "INV-CHAT-2: ttl > 1h forbidden");
        let mut nonce = [0u8; 16];
        use rand_core::RngCore;
        rand_core::OsRng.fill_bytes(&mut nonce);
        let mut tok = Self {
            session_id,
            agent_id,
            scopes,
            expires_at: now_unix + ttl_secs,
            nonce,
            sig: Vec::new(),
        };
        let sig = issuer.sign(&tok.signing_bytes());
        tok.sig = sig.to_bytes().to_vec();
        tok
    }

    /// Verify signature, ttl, scope membership. `[VERIFIED]`
    pub fn verify(
        &self,
        issuer_pub: &VerifyingKey,
        now_unix: u64,
        required: &Scope,
    ) -> Result<(), CapError> {
        if self.expires_at <= now_unix {
            return Err(CapError::Expired);
        }
        if self.sig.len() != 64 {
            return Err(CapError::BadSig);
        }
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&self.sig);
        let sig = Signature::from_bytes(&sig_bytes);
        issuer_pub
            .verify(&self.signing_bytes(), &sig)
            .map_err(|_| CapError::BadSig)?;
        if !self.scopes.contains(required) {
            return Err(CapError::ScopeMissing);
        }
        Ok(())
    }
}

/// Capability-token verification error.
#[derive(Debug, thiserror::Error)]
pub enum CapError {
    /// Token expired.
    #[error("token expired")]
    Expired,
    /// Bad signature (length, decode, or verification).
    #[error("bad signature")]
    BadSig,
    /// Required scope not in `scopes`.
    #[error("required scope missing")]
    ScopeMissing,
}

/// A tool manifest entry, signed by a publisher key. `[DERIVED from MCP 2026]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    /// Tool name (e.g. `"fetch_url"`).
    pub name: String,
    /// SHA-256 hash of the JSON schema document.
    pub schema_hash: [u8; 32],
    /// Publisher Ed25519 verifying key.
    pub publisher: [u8; 32],
    /// Ed25519 signature over `signing_bytes`.
    pub sig: Vec<u8>,
}

impl ToolManifest {
    /// Canonical bytes for signing/verification.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut h = Sha256::new();
        h.update(self.name.as_bytes());
        h.update(self.schema_hash);
        h.update(self.publisher);
        h.finalize().to_vec()
    }

    /// Sign a fresh manifest with `sk`.
    pub fn sign(name: &str, schema_hash: [u8; 32], sk: &SigningKey) -> Self {
        let publisher = sk.verifying_key().to_bytes();
        let mut m = Self {
            name: name.to_string(),
            schema_hash,
            publisher,
            sig: Vec::new(),
        };
        let sig = sk.sign(&m.signing_bytes());
        m.sig = sig.to_bytes().to_vec();
        m
    }

    /// Verify the embedded publisher signature.
    pub fn verify(&self) -> Result<(), CapError> {
        if self.sig.len() != 64 {
            return Err(CapError::BadSig);
        }
        let mut sb = [0u8; 64];
        sb.copy_from_slice(&self.sig);
        let sig = Signature::from_bytes(&sb);
        let vk = VerifyingKey::from_bytes(&self.publisher).map_err(|_| CapError::BadSig)?;
        vk.verify(&self.signing_bytes(), &sig)
            .map_err(|_| CapError::BadSig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;

    fn issuer() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    #[test]
    fn issue_and_verify_ok() {
        let iss = issuer();
        let tok = CapabilityToken::issue(
            &iss,
            [1u8; 32],
            [2u8; 32],
            vec![Scope::SendReply, Scope::ReadHistory],
            600,
            1_000_000,
        );
        assert!(tok.verify(&iss.verifying_key(), 1_000_100, &Scope::SendReply).is_ok());
    }

    #[test]
    fn expired_token_rejected() {
        let iss = issuer();
        let tok = CapabilityToken::issue(&iss, [0u8; 32], [0u8; 32], vec![Scope::SendReply], 60, 100);
        let r = tok.verify(&iss.verifying_key(), 1000, &Scope::SendReply);
        assert!(matches!(r, Err(CapError::Expired)));
    }

    #[test]
    fn scope_missing_rejected() {
        let iss = issuer();
        let tok = CapabilityToken::issue(&iss, [0u8; 32], [0u8; 32], vec![Scope::ReadHistory], 60, 100);
        let r = tok.verify(&iss.verifying_key(), 120, &Scope::SendReply);
        assert!(matches!(r, Err(CapError::ScopeMissing)));
    }

    #[test]
    #[should_panic(expected = "INV-CHAT-2")]
    fn ttl_over_1h_panics() {
        let iss = issuer();
        let _ = CapabilityToken::issue(&iss, [0u8; 32], [0u8; 32], vec![], 7200, 0);
    }

    #[test]
    fn tool_manifest_roundtrip() {
        let sk = issuer();
        let m = ToolManifest::sign("fetch_url", [9u8; 32], &sk);
        assert!(m.verify().is_ok());
    }

    #[test]
    fn tool_manifest_tamper_detected() {
        let sk = issuer();
        let mut m = ToolManifest::sign("fetch_url", [9u8; 32], &sk);
        m.name = "evil_exec".into();
        assert!(m.verify().is_err());
    }
}
