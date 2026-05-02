-- SR-03 BPB write path schema (idempotent).
-- Apply once per Neon database. Re-running this file is safe.

-- ----------------------------------------------------------------------------
-- scarabs — composite trainer state record (SR-00 Scarab)
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS scarabs (
    job_id        uuid              PRIMARY KEY,
    strategy_id   uuid              NOT NULL,
    worker_id     text,
    seed          bigint            NOT NULL,
    status        text              NOT NULL,
    created_at    timestamptz       NOT NULL DEFAULT now(),
    started_at    timestamptz,
    completed_at  timestamptz,
    best_bpb      double precision,
    best_step     bigint,
    config        jsonb             NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_scarabs_strategy ON scarabs(strategy_id);
CREATE INDEX IF NOT EXISTS idx_scarabs_status   ON scarabs(status);
CREATE INDEX IF NOT EXISTS idx_scarabs_best_bpb ON scarabs(best_bpb)
    WHERE best_bpb IS NOT NULL;

-- ----------------------------------------------------------------------------
-- bpb_samples — every BPB observation; written by SR-03 BpbWriter
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS bpb_samples (
    job_id  uuid              NOT NULL,
    step    bigint            NOT NULL,
    bpb     double precision  NOT NULL,
    ema     double precision,
    ts      timestamptz       NOT NULL DEFAULT now(),
    PRIMARY KEY (job_id, step)
);

CREATE INDEX IF NOT EXISTS idx_bpb_samples_job_ts ON bpb_samples(job_id, ts DESC);

-- ----------------------------------------------------------------------------
-- heartbeats — per-tick liveness from a worker (SR-00 Heartbeat)
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS heartbeats (
    job_id    uuid        NOT NULL,
    worker_id text        NOT NULL,
    ts        timestamptz NOT NULL,
    step      bigint,
    bpb       double precision,
    PRIMARY KEY (job_id, ts)
);

CREATE INDEX IF NOT EXISTS idx_heartbeats_worker ON heartbeats(worker_id, ts DESC);
