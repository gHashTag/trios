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

pub mod identity;
pub mod kem;
pub mod otpk;
pub mod revocation;
pub mod sealed;

pub use identity::{Identity, PrekeyBundle, PrekeyBundleBody, MLKEM_PUB_LEN, MLKEM_SEC_LEN};
pub use kem::{encapsulate_to, MlKem768Keypair, MLKEM768_CT_LEN, MLKEM768_EK_LEN, MLKEM768_SS_LEN};
pub use otpk::{JoinStrategy, Otpk, OtpkPool};
pub use revocation::{verify_identity_with_grace, RevocationCert, RevocationLedger, RevocationReason};
pub use sealed::{dest_hash, SealedEnvelope};

/// Trinity Chat protocol version this ring implements.
pub const PROTOCOL_VERSION: u16 = 1;

/// Trinity anchor identity — referenced by every gate.
pub const ANCHOR: &str = "φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA";
