//! Distance-vector routing table — mirrors MRU SRAM layout in RTL.
//!
//! 16 entries × 32 bytes = 512 bytes of SRAM (shared with VSA matmul).

use heapless::Vec;
use crate::{DestHash, Quality, MAX_ROUTES};

/// A single routing table entry.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RouteEntry {
    /// Destination node identity hash.
    pub dest: DestHash,
    /// Next-hop node identity hash.
    pub next_hop: DestHash,
    /// Hop count to destination (GF16 nibble, max 15).
    pub hops: u8,
    /// Link quality [0=best, 15=worst] — GF16 nibble.
    pub quality: Quality,
    /// Monotonic timestamp of last ANNOUNCE (cycle counter or Unix seconds).
    pub last_seen: u32,
}

/// Routing table with fixed capacity.
///
/// Heapless — no dynamic allocation, safe for no_std embedded targets.
/// The 16-entry limit maps 1:1 to the MRU SRAM block.
pub struct RoutingTable {
    entries:  Vec<RouteEntry, MAX_ROUTES>,
    self_id:  DestHash,
}

impl RoutingTable {
    /// Create a new empty routing table for this node.
    pub fn new(self_id: DestHash) -> Self {
        Self {
            entries: Vec::new(),
            self_id,
        }
    }

    /// Process an RNS ANNOUNCE packet.
    ///
    /// Updates or inserts a route if the new path is better
    /// (fewer hops or equal hops with lower quality metric).
    ///
    /// Returns `true` if the table was modified.
    pub fn process_announce(
        &mut self,
        dest:    DestHash,
        via:     DestHash,
        hops:    u8,
        quality: Quality,
        now:     u32,
    ) -> bool {
        // Enforce GF16 discipline: clamp to 4-bit values
        let hops = hops    & 0x0F;
        let q    = quality & 0x0F;

        for e in self.entries.iter_mut() {
            if e.dest == dest {
                let better_hops    = hops < e.hops;
                let same_hops_bqlt = hops == e.hops && q < e.quality;
                if better_hops || same_hops_bqlt {
                    e.next_hop  = via;
                    e.hops      = hops;
                    e.quality   = q;
                    e.last_seen = now;
                    return true;
                }
                return false;
            }
        }
        // New destination — insert if capacity allows
        let entry = RouteEntry { dest, next_hop: via, hops, quality: q, last_seen: now };
        self.entries.push(entry).is_ok()
    }

    /// Look up the next hop for a destination hash.
    ///
    /// Returns `None` if the destination is this node itself
    /// (deliver locally) or if no route is known.
    pub fn next_hop(&self, dest: &DestHash) -> Option<DestHash> {
        if dest == &self.self_id {
            return None; // local delivery signal
        }
        self.entries
            .iter()
            .find(|e| &e.dest == dest)
            .map(|e| e.next_hop)
    }

    /// Expire routes older than `ttl` time units.
    pub fn expire(&mut self, now: u32, ttl: u32) {
        self.entries.retain(|e| now.wrapping_sub(e.last_seen) < ttl);
    }

    /// Number of active routes.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the table has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> DestHash { [byte; 16] }

    #[test]
    fn insert_and_lookup() {
        let self_id  = id(0x00);
        let dest     = id(0x01);
        let via      = id(0x02);
        let mut tbl  = RoutingTable::new(self_id);

        assert!(tbl.process_announce(dest, via, 2, 3, 100));
        assert_eq!(tbl.next_hop(&dest), Some(via));
    }

    #[test]
    fn local_delivery() {
        let self_id = id(0xAB);
        let tbl     = RoutingTable::new(self_id);
        assert_eq!(tbl.next_hop(&self_id), None);
    }

    #[test]
    fn gf16_quality_clamp() {
        let mut tbl = RoutingTable::new(id(0x00));
        tbl.process_announce(id(0x01), id(0x02), 0xFF, 0xFF, 0);
        let e = tbl.entries[0];
        assert_eq!(e.hops,    0x0F);  // clamped to GF16 nibble
        assert_eq!(e.quality, 0x0F);  // clamped to GF16 nibble
    }

    #[test]
    fn expiry() {
        let mut tbl = RoutingTable::new(id(0x00));
        tbl.process_announce(id(0x01), id(0x02), 1, 0, 0);
        tbl.expire(1000, 500);  // now=1000, ttl=500 ⇒ 0 < 500 ... expired
        assert!(tbl.is_empty());
    }

    #[test]
    fn prefer_shorter_path() {
        let mut tbl = RoutingTable::new(id(0x00));
        let dest    = id(0x01);
        tbl.process_announce(dest, id(0x10), 5, 1, 0);
        // Better path via id(0x20) with 3 hops
        assert!(tbl.process_announce(dest, id(0x20), 3, 2, 1));
        assert_eq!(tbl.next_hop(&dest), Some(id(0x20)));
    }
}
