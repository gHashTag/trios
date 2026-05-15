//! # CR-CHAT-01 — identity + sealed sender
//!
//! L-CHAT-1 (trinity-fpga#29) + L-CHAT-4 (trinity-fpga#32).
//!
//! Two tightly-coupled chat primitives live here:
//!
//! - [`identity`] — Ed25519 long-term + X25519 prekey + ML-KEM-768
//!   prekey-bundle skeleton (R-CHAT-2 hybrid PQ, R-CHAT-4 sign only the
//!   bundle).
//! - [`sealed`] — sealed-sender envelope over `trios-mesh` (R-CHAT-3:
//!   the mesh sees only `(dest_hash[16], padded_envelope)`).
//!
//! Both modules are pure Silver-tier: they work on byte arrays and
//! `x25519-dalek` / `ed25519-dalek` types, no I/O, no async, no
//! storage.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
//!
//! ## Honesty (R5)
//!
//! - `[VERIFIED]` — `Identity::generate`, `PrekeyBundle::verify`,
//!   `SealedEnvelope::seal`/`unseal` round-trip + 5 falsifier tests.
//! - `[ASPIRATIONAL]` — ML-KEM-768 public bytes are still a SHA-256
//!   placeholder; concrete `ml-kem` integration lands in CR-CHAT-02.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod appack_replay;
pub mod handshake_fingerprint;
pub mod identity;
pub mod kem;
pub mod kem_decap_oracle;
pub mod keypackage_init_key_reuse;
pub mod otpk;
pub mod prekey_signature_chain;
pub mod revocation;
pub mod sealed;
pub mod welcome_encrypted_group_info_aead;

pub use appack_replay::{
    AppAckError, AppAckLeaf, AppAckLedger, AppAckProposal, Generation, MessageRange,
};

pub use handshake_fingerprint::{
    HandshakeError, HandshakeFingerprint, HSF_DOMAIN, HSF_LEN,
};
pub use identity::{Identity, PrekeyBundle, PrekeyBundleBody, MLKEM_PUB_LEN, MLKEM_SEC_LEN};
pub use kem::{encapsulate_to, MlKem768Keypair, MLKEM768_CT_LEN, MLKEM768_EK_LEN, MLKEM768_SS_LEN};
pub use kem_decap_oracle::{observe as observe_decap, ss_eq as decap_ss_eq, DecapObservation, KEM_DECAP_ORACLE_CT_LEN, KEM_DECAP_ORACLE_SS_LEN};
pub use keypackage_init_key_reuse::{
    validate_keypackage_init_key, KeyPackage, KeyPackageInitKeyError, KeyPackageView,
    KEYPACKAGE_INIT_KEY_LEN,
};
pub use otpk::{JoinStrategy, Otpk, OtpkPool};
pub use prekey_signature_chain::{
    validate_prekey_chain, ChainBindingTag, PrekeyChainBundle, PrekeyChainError, PrekeyChainKey,
    PrekeyChainView,
};
pub use revocation::{verify_identity_with_grace, RevocationCert, RevocationLedger, RevocationReason};
pub use sealed::{dest_hash, SealedEnvelope};
pub use welcome_encrypted_group_info_aead::{
    validate_welcome_aead_envelope, WelcomeAeadEnvelope, WelcomeAeadError, WelcomeAeadView,
    WELCOME_GROUP_INFO_AEAD_NONCE_LEN, WELCOME_GROUP_INFO_MIN_CT_LEN,
};

/// Trinity Chat protocol version this ring implements.
pub const PROTOCOL_VERSION: u16 = 1;

/// Trinity anchor identity — referenced by every gate.
pub const ANCHOR: &str = "φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA";
