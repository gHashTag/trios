-- L-E2E-4 · trinity-fpga#26 · trios-mesh-node route table persistence
-- Anchor: φ² + φ⁻² = 3
--
-- Schema for route_table mirrored from in-memory `trios_mesh::routing::RoutingTable`.
-- The daemon writes every accepted announce here and reloads on boot, collapsing
-- post-restart convergence from 30–120 s to < 5 s.

CREATE TABLE IF NOT EXISTS route_table (
    -- Local node identity that owns this row (multi-tenant safe).
    self_dest_hash BYTEA       NOT NULL,
    -- Destination this route points at. dest_hash = SHA256(x25519_pubkey)[..16].
    dest_hash      BYTEA       NOT NULL,
    -- Next hop (also a 16-byte dest_hash).
    next_hop       BYTEA       NOT NULL,
    -- GF16-clamped 4-bit nibbles, exposed as smallint for clarity.
    hops           SMALLINT    NOT NULL CHECK (hops    BETWEEN 0 AND 15),
    quality        SMALLINT    NOT NULL CHECK (quality BETWEEN 0 AND 15),
    -- Optional X25519 pubkey (32 bytes) — populated by L-E2E-3 announce path.
    pub_key        BYTEA,
    -- Wall-clock timestamps for TTL eviction and audit.
    last_seen      TIMESTAMPTZ NOT NULL DEFAULT now(),
    announced_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (self_dest_hash, dest_hash)
);

-- Fast TTL sweep: WHERE last_seen < now() - INTERVAL '24 hours'.
CREATE INDEX IF NOT EXISTS idx_route_table_last_seen
    ON route_table (last_seen);

-- Quick lookup of all routes owned by one local node (boot-time reload).
CREATE INDEX IF NOT EXISTS idx_route_table_self
    ON route_table (self_dest_hash);
