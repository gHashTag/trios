-- bpb_samples: per-cell BPB result rows produced by tier runners
-- Single source of truth for matrix #446 coverage progress.
-- Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877
CREATE SCHEMA IF NOT EXISTS ssot;

CREATE TABLE IF NOT EXISTS ssot.bpb_samples (
    id              BIGSERIAL PRIMARY KEY,
    cell_id         INTEGER NOT NULL,
    tier            TEXT NOT NULL,
    seed            INTEGER NOT NULL,
    bpb             DOUBLE PRECISION NOT NULL,
    steps           INTEGER NOT NULL,
    sha_pin         TEXT NOT NULL,
    runner_service  TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (cell_id, seed, sha_pin)
);

CREATE INDEX IF NOT EXISTS bpb_samples_cell_idx ON ssot.bpb_samples (cell_id);
CREATE INDEX IF NOT EXISTS bpb_samples_tier_idx ON ssot.bpb_samples (tier);
CREATE INDEX IF NOT EXISTS bpb_samples_created_idx ON ssot.bpb_samples (created_at DESC);
