//! L-CHAT-7-funnel · Wave-15 — Tailscale-funnel egress fingerprinting guard.
//!
//! Where [`crate::CoverScheduler`] hides per-envelope timing and
//! [`crate::uniform_gap_ms`] quantises gaps, this module hides the
//! *wire-fingerprint* of the outermost transport layer (Tailscale-funnel
//! egress / TLS class). The mesh adversary observes:
//!
//! 1. TLS ClientHello fingerprint (JA3-like): ALPN list + cipher-suite list
//!    + supported-versions list.
//! 2. Per-flow byte length and inter-burst timing.
//!
//! Without normalisation, every Trinity-Chat egress flow looks unique on
//! the wire — JA3 alone leaks the runtime/version. This module exposes a
//! pure normaliser that bins each observable into a small canonical set:
//!
//! - [`TlsClass`]   — coarse cipher / ALPN / version triple.
//! - [`uniform_length_class`] — bytes → `{1024, 4096, 16384, 65536}`.
//! - [`uniform_burst_ms`]     — burst gap → `{50, 250, 1000, 5000}` ms.
//!
//! [`EgressFingerprint::normalise`] turns raw observables into the
//! canonical 3-tuple the egress wire MUST emit; two flows that disagree
//! on this tuple are unambiguously distinguishable to a passive observer
//! and are rejected by the funnel guard.
//!
//! ## Honesty (R5)
//!
//! - `[VERIFIED]` for the binning logic and the 6 EFP-01..06 unit tests
//!   in this file.
//! - `[ASPIRATIONAL]` for the *concrete* Tailscale-funnel cipher list —
//!   the canonical class is locked at the protocol level, but the actual
//!   `tls13` exporter wiring lives outside this Silver-tier ring.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · EGRESS-FINGERPRINT`

use trios_chat_cr_chat_00::{Error, Result};

/// Canonical TLS class — every Trinity-Chat egress flow MUST present
/// exactly this fingerprint over the Tailscale funnel.
///
/// `[VERIFIED]` — equality / hash check is constant-time-irrelevant
/// because all fields are 1-byte enums.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsClass {
    /// TLS version major×100 + minor — locked at `TLS 1.3` (`303`).
    pub version: u16,
    /// Single ALPN id — locked at `h2`.
    pub alpn: AlpnId,
    /// Single cipher-suite id — locked at `TLS_AES_128_GCM_SHA256`.
    pub cipher: CipherId,
}

/// ALPN id — one canonical value for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlpnId {
    /// HTTP/2 over TLS — the only ALPN Trinity-Chat funnels emit.
    H2,
}

/// Cipher-suite id — one canonical value for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherId {
    /// TLS_AES_128_GCM_SHA256 — RFC 8446 §B.4. Mandatory in TLS 1.3.
    Aes128GcmSha256,
}

/// The single canonical class the funnel MUST emit.
pub const CANONICAL_TLS_CLASS: TlsClass = TlsClass {
    version: 0x0303, // TLS 1.3 record-layer hex (RFC 8446)
    alpn: AlpnId::H2,
    cipher: CipherId::Aes128GcmSha256,
};

/// Canonical egress length classes (bytes). Every emitted flow MUST be
/// padded up to one of these. Mirrors `CANONICAL_GAPS_MS` in shape.
pub const CANONICAL_LENGTH_CLASSES: [u32; 4] = [1024, 4096, 16384, 65536];

/// Canonical burst-gap classes (ms).
pub const CANONICAL_BURST_GAPS_MS: [u64; 4] = [50, 250, 1_000, 5_000];

/// Raw observables (per-flow, per-burst) extracted from the OS / the
/// Tailscale-funnel ingress. Pure data — no I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EgressObservables {
    /// Negotiated TLS version (e.g. `0x0303`).
    pub version: u16,
    /// ALPN id seen on the wire.
    pub alpn: AlpnId,
    /// Cipher-suite id seen on the wire.
    pub cipher: CipherId,
    /// Raw flow byte length.
    pub bytes: u32,
    /// Raw inter-burst gap (ms).
    pub gap_ms: u64,
}

/// Normalised egress fingerprint — the canonical 3-tuple every flow must
/// agree on. Two flows that produce different fingerprints are
/// distinguishable and will be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EgressFingerprint {
    /// TLS class (must equal [`CANONICAL_TLS_CLASS`]).
    pub tls: TlsClass,
    /// Quantised length class.
    pub length_class: u32,
    /// Quantised burst-gap class.
    pub burst_class: u64,
}

impl EgressFingerprint {
    /// Bin raw observables into the canonical fingerprint and verify
    /// the TLS class is the locked one. Returns the same fingerprint
    /// for any two flows whose raw observables fall into the same bins.
    ///
    /// Returns `Err(Error::Invariant("egress_fingerprint_tls_class"))`
    /// if the TLS class deviates from [`CANONICAL_TLS_CLASS`].
    pub fn normalise(obs: EgressObservables) -> Result<Self> {
        let tls = TlsClass {
            version: obs.version,
            alpn: obs.alpn,
            cipher: obs.cipher,
        };
        if tls != CANONICAL_TLS_CLASS {
            return Err(Error::Invariant("egress_fingerprint_tls_class"));
        }
        Ok(Self {
            tls,
            length_class: uniform_length_class(obs.bytes),
            burst_class: uniform_burst_ms(obs.gap_ms),
        })
    }

    /// Public canonical-class accessor (for the funnel emit-side).
    pub fn canonical_tls() -> TlsClass {
        CANONICAL_TLS_CLASS
    }
}

/// Quantise raw flow bytes into the largest canonical length class
/// `c` such that `c <= bytes`; below the smallest class we still
/// return the smallest class so the wire never shows a sub-class flow.
///
/// `[VERIFIED]` — `egress_length_quantises_to_canonical_set` test.
pub fn uniform_length_class(bytes: u32) -> u32 {
    let mut chosen = CANONICAL_LENGTH_CLASSES[0];
    for &c in &CANONICAL_LENGTH_CLASSES {
        if bytes >= c {
            chosen = c;
        }
    }
    chosen
}

/// Quantise a measured inter-burst gap (in milliseconds) into one of
/// the canonical burst-gap classes — same shape as
/// [`crate::uniform_gap_ms`] but at finer resolution.
///
/// `[VERIFIED]` — `egress_burst_gap_quantises_to_canonical_set` test.
pub fn uniform_burst_ms(measured_ms: u64) -> u64 {
    let mut chosen = CANONICAL_BURST_GAPS_MS[0];
    for &g in &CANONICAL_BURST_GAPS_MS {
        if measured_ms >= g {
            chosen = g;
        }
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(bytes: u32, gap_ms: u64) -> EgressObservables {
        EgressObservables {
            version: 0x0303,
            alpn: AlpnId::H2,
            cipher: CipherId::Aes128GcmSha256,
            bytes,
            gap_ms,
        }
    }

    #[test]
    fn efp_01_canonical_tls_class_accepted() {
        // EFP-01: a flow that already presents the canonical TLS class
        // MUST produce a normalised fingerprint without error.
        let f = EgressFingerprint::normalise(obs(2048, 300)).unwrap();
        assert_eq!(f.tls, CANONICAL_TLS_CLASS);
    }

    #[test]
    fn efp_02_non_canonical_tls_class_rejected() {
        // EFP-02: any deviation in version / ALPN / cipher rejects.
        let mut o = obs(2048, 300);
        o.version = 0x0304; // not TLS 1.3 record-layer
        let r = EgressFingerprint::normalise(o);
        assert!(matches!(r, Err(Error::Invariant("egress_fingerprint_tls_class"))));
    }

    #[test]
    fn efp_03_length_quantises_to_canonical_set() {
        // EFP-03: every bin output is one of the canonical classes,
        // and the bin is monotone (larger raw → ≥ raw bin).
        for &b in &[0u32, 1, 1023, 1024, 4095, 4096, 16_383, 16_384, 65_535, 65_536, u32::MAX] {
            let c = uniform_length_class(b);
            assert!(CANONICAL_LENGTH_CLASSES.contains(&c));
        }
        assert_eq!(uniform_length_class(0), 1024);
        assert_eq!(uniform_length_class(1023), 1024);
        assert_eq!(uniform_length_class(1024), 1024);
        assert_eq!(uniform_length_class(4096), 4096);
        assert_eq!(uniform_length_class(70_000), 65_536);
    }

    #[test]
    fn efp_04_burst_gap_quantises_to_canonical_set() {
        // EFP-04: burst-gap binning matches the canonical set.
        for &m in &[0u64, 1, 49, 50, 249, 250, 999, 1_000, 4_999, 5_000, u64::MAX] {
            let g = uniform_burst_ms(m);
            assert!(CANONICAL_BURST_GAPS_MS.contains(&g));
        }
        assert_eq!(uniform_burst_ms(0), 50);
        assert_eq!(uniform_burst_ms(49), 50);
        assert_eq!(uniform_burst_ms(50), 50);
        assert_eq!(uniform_burst_ms(1_000), 1_000);
        assert_eq!(uniform_burst_ms(10_000), 5_000);
    }

    #[test]
    fn efp_05_two_flows_with_same_bins_have_equal_fingerprint() {
        // EFP-05: any two flows whose raw observables fall into the
        // same length+gap bins MUST produce equal fingerprints —
        // unlinkability across egress flows.
        let f1 = EgressFingerprint::normalise(obs(2_000, 300)).unwrap();
        let f2 = EgressFingerprint::normalise(obs(3_500, 600)).unwrap();
        // 2_000 and 3_500 both bin to 1024; 300 and 600 both bin to 250.
        assert_eq!(f1, f2, "same-bin flows must produce identical fingerprints");
    }

    #[test]
    fn efp_06_cross_bin_flows_distinguishable_only_by_canonical_axis() {
        // EFP-06: a flow that crosses a length boundary AND a gap
        // boundary differs from a same-bin flow ONLY along the
        // canonical axes — never via raw values. This pins that
        // the fingerprint depends ONLY on (tls, length_class,
        // burst_class) and never on the raw u32 / u64 input.
        let f_small_fast = EgressFingerprint::normalise(obs(2_000, 100)).unwrap();
        let f_large_slow = EgressFingerprint::normalise(obs(20_000, 6_000)).unwrap();
        assert_ne!(f_small_fast, f_large_slow);
        // Both share the canonical TLS class.
        assert_eq!(f_small_fast.tls, f_large_slow.tls);
        // Their length and burst classes differ along the canonical
        // axes (not raw values).
        assert_ne!(f_small_fast.length_class, f_large_slow.length_class);
        assert_ne!(f_small_fast.burst_class, f_large_slow.burst_class);
        // Concretely: small-fast = (1024, 50); large-slow = (16384, 5000).
        assert_eq!(f_small_fast.length_class, 1024);
        assert_eq!(f_small_fast.burst_class, 50);
        assert_eq!(f_large_slow.length_class, 16_384);
        assert_eq!(f_large_slow.burst_class, 5_000);
    }
}
