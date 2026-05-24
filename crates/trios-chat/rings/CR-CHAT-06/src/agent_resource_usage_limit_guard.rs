//! # CR-CHAT-06 — Agent resource usage limit guard (Wave-117 Lane B)
//!
//! AGENT SAFETY — per-session CPU/memory/disk quotas must not be
//! exceeded; runaway agents can exhaust host resources.
//!
//! Without resource limits, a compromised or misconfigured agent can:
//!
//! * **CPU exhaustion** — infinite loops or expensive computations
//!   starve other sessions and the host process.
//! * **Memory exhaustion** — unbounded allocations cause OOM kills,
//!   taking down the entire process.
//! * **Disk exhaustion** — unbounded writes fill storage, causing
//!   data loss for all sessions.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. CPU ms <= `ARUL_MAX_CPU_MS`.
//! 2. Memory bytes <= `ARUL_MAX_MEMORY`.
//! 3. Disk bytes <= `ARUL_MAX_DISK`.
//! 4. Session ID must not be zero.
//! 5. No duplicate session IDs.
//! 6. Total records <= `ARUL_MAX_RECORDS`.
//!
//! Tests **ARUL-01..10**. Error enum [`ResourceLimitError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RESOURCE-BOUND`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum CPU time per session in milliseconds.
pub const ARUL_MAX_CPU_MS: u64 = 30_000;

/// Maximum memory per session in bytes.
pub const ARUL_MAX_MEMORY: u64 = 512 * 1024 * 1024;

/// Maximum disk usage per session in bytes.
pub const ARUL_MAX_DISK: u64 = 1024 * 1024 * 1024;

/// Maximum records per batch.
pub const ARUL_MAX_RECORDS: usize = 1024;

/// Session ID length.
pub const ARUL_SESSION_ID_LEN: usize = 32;

/// A resource usage record for one session.
#[derive(Debug, Clone)]
pub struct ResourceRecord {
    /// Session identifier.
    pub session_id: [u8; ARUL_SESSION_ID_LEN],
    /// CPU time consumed in milliseconds.
    pub cpu_ms: u64,
    /// Memory consumed in bytes.
    pub memory_bytes: u64,
    /// Disk space consumed in bytes.
    pub disk_bytes: u64,
}

/// All ways resource usage validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceLimitError {
    /// CPU limit exceeded.
    CpuExceeded { idx: usize, got: u64, max: u64 },
    /// Memory limit exceeded.
    MemoryExceeded { idx: usize, got: u64, max: u64 },
    /// Disk limit exceeded.
    DiskExceeded { idx: usize, got: u64, max: u64 },
    /// Zero session ID.
    ZeroSession(usize),
    /// Duplicate session ID.
    DuplicateSession(usize),
    /// Too many records.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent resource usage limits.
pub fn validate_resource_limits(
    records: &[ResourceRecord],
) -> Result<(), ResourceLimitError> {
    if records.len() > ARUL_MAX_RECORDS {
        return Err(ResourceLimitError::TooMany {
            got: records.len(),
            max: ARUL_MAX_RECORDS,
        });
    }
    let mut seen: BTreeSet<[u8; ARUL_SESSION_ID_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.session_id == [0u8; ARUL_SESSION_ID_LEN] {
            return Err(ResourceLimitError::ZeroSession(i));
        }
        if !seen.insert(r.session_id) {
            return Err(ResourceLimitError::DuplicateSession(i));
        }
        if r.cpu_ms > ARUL_MAX_CPU_MS {
            return Err(ResourceLimitError::CpuExceeded {
                idx: i,
                got: r.cpu_ms,
                max: ARUL_MAX_CPU_MS,
            });
        }
        if r.memory_bytes > ARUL_MAX_MEMORY {
            return Err(ResourceLimitError::MemoryExceeded {
                idx: i,
                got: r.memory_bytes,
                max: ARUL_MAX_MEMORY,
            });
        }
        if r.disk_bytes > ARUL_MAX_DISK {
            return Err(ResourceLimitError::DiskExceeded {
                idx: i,
                got: r.disk_bytes,
                max: ARUL_MAX_DISK,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; ARUL_SESSION_ID_LEN] {
        [byte; ARUL_SESSION_ID_LEN]
    }

    fn record(session: u8, cpu: u64, mem: u64, disk: u64) -> ResourceRecord {
        ResourceRecord { session_id: sid(session), cpu_ms: cpu, memory_bytes: mem, disk_bytes: disk }
    }

    fn valid_records() -> Vec<ResourceRecord> {
        vec![
            record(0x01, 100, 1024, 2048),
            record(0x02, 200, 2048, 4096),
        ]
    }

    /// **ARUL-01** — CPU exceeded rejected.
    #[test]
    fn arul_01_cpu_exceeded_rejected() {
        let rs = vec![record(0x01, ARUL_MAX_CPU_MS + 1, 1024, 2048)];
        assert_eq!(
            validate_resource_limits(&rs),
            Err(ResourceLimitError::CpuExceeded {
                idx: 0,
                got: ARUL_MAX_CPU_MS + 1,
                max: ARUL_MAX_CPU_MS,
            })
        );
    }

    /// **ARUL-02** — memory exceeded rejected.
    #[test]
    fn arul_02_memory_exceeded_rejected() {
        let rs = vec![record(0x01, 100, ARUL_MAX_MEMORY + 1, 2048)];
        assert_eq!(
            validate_resource_limits(&rs),
            Err(ResourceLimitError::MemoryExceeded {
                idx: 0,
                got: ARUL_MAX_MEMORY + 1,
                max: ARUL_MAX_MEMORY,
            })
        );
    }

    /// **ARUL-03** — disk exceeded rejected.
    #[test]
    fn arul_03_disk_exceeded_rejected() {
        let rs = vec![record(0x01, 100, 1024, ARUL_MAX_DISK + 1)];
        assert_eq!(
            validate_resource_limits(&rs),
            Err(ResourceLimitError::DiskExceeded {
                idx: 0,
                got: ARUL_MAX_DISK + 1,
                max: ARUL_MAX_DISK,
            })
        );
    }

    /// **ARUL-04** — zero session rejected.
    #[test]
    fn arul_04_zero_session_rejected() {
        let rs = vec![ResourceRecord { session_id: [0u8; ARUL_SESSION_ID_LEN], cpu_ms: 100, memory_bytes: 1024, disk_bytes: 2048 }];
        assert_eq!(
            validate_resource_limits(&rs),
            Err(ResourceLimitError::ZeroSession(0))
        );
    }

    /// **ARUL-05** — duplicate session rejected.
    #[test]
    fn arul_05_duplicate_session_rejected() {
        let rs = vec![
            record(0x01, 100, 1024, 2048),
            record(0x01, 200, 2048, 4096),
        ];
        assert_eq!(
            validate_resource_limits(&rs),
            Err(ResourceLimitError::DuplicateSession(1))
        );
    }

    /// **ARUL-06** — too many rejected.
    #[test]
    fn arul_06_too_many_rejected() {
        let rs: Vec<ResourceRecord> = (0..=ARUL_MAX_RECORDS)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                let mut session = [0u8; ARUL_SESSION_ID_LEN];
                session[0] = b;
                ResourceRecord { session_id: session, cpu_ms: 100, memory_bytes: 1024, disk_bytes: 2048 }
            })
            .collect();
        assert_eq!(
            validate_resource_limits(&rs),
            Err(ResourceLimitError::TooMany {
                got: ARUL_MAX_RECORDS + 1,
                max: ARUL_MAX_RECORDS,
            })
        );
    }

    /// **ARUL-07** — valid accepted.
    #[test]
    fn arul_07_valid_accepted() {
        assert_eq!(validate_resource_limits(&valid_records()), Ok(()));
    }

    /// **ARUL-08** — empty accepted.
    #[test]
    fn arul_08_empty_accepted() {
        assert_eq!(validate_resource_limits(&[]), Ok(()));
    }

    /// **ARUL-09** — boundary CPU accepted.
    #[test]
    fn arul_09_boundary_cpu_accepted() {
        let rs = vec![record(0x01, ARUL_MAX_CPU_MS, ARUL_MAX_MEMORY, ARUL_MAX_DISK)];
        assert_eq!(validate_resource_limits(&rs), Ok(()));
    }

    /// **ARUL-10** — many sessions accepted.
    #[test]
    fn arul_10_many_sessions_accepted() {
        let rs: Vec<ResourceRecord> = (0..100)
            .map(|i| {
                let mut session = [0u8; ARUL_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                session[0..8].copy_from_slice(&val.to_be_bytes());
                ResourceRecord { session_id: session, cpu_ms: 100, memory_bytes: 1024, disk_bytes: 2048 }
            })
            .collect();
        assert_eq!(validate_resource_limits(&rs), Ok(()));
    }
}
