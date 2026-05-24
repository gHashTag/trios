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
pub mod ephemeral_mailbox_unlinkability;
pub mod handshake_fingerprint;
pub mod identity;
pub mod kem;
pub mod kem_decap_oracle;
pub mod key_package_lifetime_grace_window_expiry;
pub mod keypackage_init_key_reuse;
pub mod keypackage_capabilities_binding;
pub mod otpk;
pub mod prekey_signature_chain;
pub mod epoch_advancement_guard;
pub mod prekey_bundle_expiry_guard;
pub mod prekey_signature_algorithm_downgrade;
pub mod prekey_bundle_freshness_guard;
pub mod identity_key_rotation_guard;
pub mod prekey_bundle_binding_guard;
pub mod prekey_signature_nonce_freshness;
pub mod revocation;
pub mod sealed;
pub mod welcome_encrypted_group_info_aead;

pub use appack_replay::{
    AppAckError, AppAckLeaf, AppAckLedger, AppAckProposal, Generation, MessageRange,
};

pub use ephemeral_mailbox_unlinkability::{
    validate_ephemeral_mailbox_envelope, EphemeralMailboxEnvelope, EphemeralMailboxError,
    EphemeralMailboxView, ENVELOPE_BINDING_TAG_LEN, EPHEMERAL_MAILBOX_TOKEN_LEN,
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
pub use epoch_advancement_guard::{
    validate_epoch_chain, validate_epoch_transition, EpochAdvanceError, EpochTransition,
    EPOCH_MAX_GAP, EPOCH_MAX_VALUE,
};
pub use prekey_signature_algorithm_downgrade::{
    validate_signature_bundle, validate_signature_entry, SigAlgoDowngradeError, SignatureEntry,
    ALGO_ED25519, APPROVED_ALGOS, ED25519_PK_LEN, ED25519_SIG_LEN,
};
pub use prekey_bundle_freshness_guard::{
    validate_bundle_batch, validate_bundle_freshness, BundleCheck, BundleFreshnessError,
    PBFG_MAX_AGE_MS, PBFG_MAX_BUNDLES, PBFG_VERSION,
};
pub use identity_key_rotation_guard::{
    validate_id_key_rotations, IdKeyRotation, IdKeyRotationError, IKRG_KEY_LEN, IKRG_MAX_ROTATIONS,
};
pub use prekey_bundle_binding_guard::{
    BundleTracker, PrekeyBundleBinding, BundleBindingError,
    PKBB_ID_LEN, PKBB_KEM_LEN, PKBB_MAX_TRACKED, PKBB_PREKEY_LEN,
};
pub use prekey_signature_nonce_freshness::{
    NonceTracker, NonceFreshnessError, PSNF_MAX_TRACKED, PSNF_NONCE_LEN,
};
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

pub mod key_package_expiry_guard;
pub use key_package_expiry_guard::{
    validate_key_package_expiry, KeyPackageExpiryError, KPX_MAX_LIFETIME_SECS, KPX_MIN_LIFETIME_SECS,
};

pub mod credential_chain_path_length_guard;
pub use credential_chain_path_length_guard::{
    validate_credential_chain, Credential, CredentialChainError, CCPL_MAX_DEPTH,
};

pub mod signature_algorithm_pinning_guard;
pub use signature_algorithm_pinning_guard::{
    validate_sig_algo_pinning, SigAlgoPinError, SAPN_ALLOWED_ALGOS, SAPN_MAX_PINS,
};

pub mod prekey_bundle_one_time_use_guard;
pub use prekey_bundle_one_time_use_guard::{
    validate_prekey_one_time, PrekeyReuseError, PBOU_MAX_BUNDLE, PBOU_MAX_CONSUMED,
};

pub mod identity_key_usage_count_guard;
pub use identity_key_usage_count_guard::{
    key_needs_rotation, validate_key_usage, KeyUsageError,
    IKUC_MAX_SIGNATURES, IKUC_WARN_SIGNATURES,
};

/// Trinity Chat protocol version this ring implements.
pub const PROTOCOL_VERSION: u16 = 1;

/// Trinity anchor identity — referenced by every gate.
pub const ANCHOR: &str = "φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA";
