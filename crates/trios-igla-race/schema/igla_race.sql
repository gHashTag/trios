-- IGLA RACE: Failure Memory Protocol Schema
--
-- 3 tables:
-- 1. igla_race_trials     — each trial with BPB at each rung + status
-- 2. igla_race_experience — failure lessons (what NOT to do)
-- 3. igla_leaderboard     VIEW — combined, sorted by BPB
--
-- Usage:
--   psql $NEON_DATABASE_URL -f crates/trios-igla-race/schema/igla_race.sql

-- ============================================================================
-- TABLE 1: igla_race_trials — Trial Tracking
-- ============================================================================

CREATE TABLE IF NOT EXISTS igla_race_trials (
    trial_id TEXT PRIMARY KEY,
    machine_id TEXT NOT NULL,
    worker_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',  -- 'running', 'pruned', 'complete', 'igla_found', 'error'
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    -- Hyperparameters (stored as JSON for flexibility)
    config JSONB NOT NULL,

    -- ASHA rung results
    rung_1_steps INT DEFAULT 1000,
    rung_1_bpb FLOAT,
    rung_1_at TIMESTAMPTZ,

    rung_2_steps INT DEFAULT 3000,
    rung_2_bpb FLOAT,
    rung_2_at TIMESTAMPTZ,

    rung_3_steps INT DEFAULT 9000,
    rung_3_bpb FLOAT,
    rung_3_at TIMESTAMPTZ,

    rung_4_steps INT DEFAULT 27000,
    rung_4_bpb FLOAT,
    rung_4_at TIMESTAMPTZ,

    -- Final results
    best_bpb FLOAT,
    best_step INT,
    pruned_at_step INT,
    completed_at TIMESTAMPTZ,

    -- Derived from config (for indexing)
    d_model INT,
    lr FLOAT,
    wd FLOAT,
    n_gram INT
);

-- ============================================================================
-- TABLE 2: igla_race_experience — Failure Memory Protocol
-- ============================================================================

CREATE TABLE IF NOT EXISTS igla_race_experience (
    id SERIAL PRIMARY KEY,
    trial_id TEXT NOT NULL REFERENCES igla_race_trials(trial_id) ON DELETE CASCADE,
    outcome TEXT NOT NULL,  -- 'pruned', 'complete', 'igla_found', 'error'

    -- What failed and why
    pruned_at_rung INT,
    pruned_bpb FLOAT,
    pruned_reason TEXT,

    -- Auto-generated lesson
    lesson TEXT NOT NULL,
    lesson_type TEXT NOT NULL,  -- 'WARN', 'PATTERN', 'SUCCESS', 'TIP'
    confidence FLOAT DEFAULT 1.0,  -- 0.0 to 1.0, based on how many trials confirm

    -- Pattern matching
    pattern_key TEXT,  -- e.g., "lr_gt_0.05", "d_model_lt_64", "bpb_gt_3_at_rung1"
    pattern_count INT DEFAULT 1,  -- how many times this pattern failed

    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- INDEXES for Performance
-- ============================================================================

-- Trial indexes
CREATE INDEX IF NOT EXISTS idx_trials_status ON igla_race_trials(status);
CREATE INDEX IF NOT EXISTS idx_trials_machine ON igla_race_trials(machine_id);
CREATE INDEX IF NOT EXISTS idx_trials_created ON igla_race_trials(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_trials_best_bpb ON igla_race_trials(best_bpb) WHERE best_bpb IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_trials_d_model ON igla_race_trials(d_model);
CREATE INDEX IF NOT EXISTS idx_trials_lr ON igla_race_trials(lr);

-- Experience indexes
CREATE INDEX IF NOT EXISTS idx_experience_outcome ON igla_race_experience(outcome);
CREATE INDEX IF NOT EXISTS idx_experience_lesson_type ON igla_race_experience(lesson_type);
CREATE INDEX IF NOT EXISTS idx_experience_pattern ON igla_race_experience(pattern_key);
CREATE INDEX IF NOT EXISTS idx_experience_created ON igla_race_experience(created_at DESC);

-- Prevent duplicate running trials for same config
CREATE UNIQUE INDEX IF NOT EXISTS idx_trials_running_config
    ON igla_race_trials ((config::text))
    WHERE status = 'running';

-- ============================================================================
-- TABLE 3: igla_leaderboard VIEW
-- ============================================================================

CREATE OR REPLACE VIEW igla_leaderboard AS
SELECT
    ROW_NUMBER() OVER (ORDER BY
        CASE
            WHEN t.status = 'igla_found' THEN 0
            WHEN t.status = 'complete' THEN 1
            WHEN t.status = 'running' THEN 2
            WHEN t.status = 'pruned' THEN 3
            ELSE 4
        END,
        COALESCE(t.best_bpb, 999.999)
    ) as rank,

    t.trial_id,
    t.machine_id,
    t.status,
    t.best_bpb,
    t.best_step,
    t.d_model,
    t.lr,
    t.wd,
    t.n_gram,
    t.pruned_at_step,
    t.created_at,
    t.completed_at,

    -- Top lesson if pruned
    (
        SELECT e.lesson
        FROM igla_race_experience e
        WHERE e.trial_id = t.trial_id AND e.outcome = 'pruned'
        LIMIT 1
    ) as pruned_lesson

FROM igla_race_trials t
ORDER BY
    CASE
        WHEN t.status = 'igla_found' THEN 0
        WHEN t.status = 'complete' THEN 1
        WHEN t.status = 'running' THEN 2
        WHEN t.status = 'pruned' THEN 3
        ELSE 4
    END,
    COALESCE(t.best_bpb, 999.999)
LIMIT 100;

-- ============================================================================
-- VIEW: Failure Patterns Summary
-- ============================================================================

CREATE OR REPLACE VIEW v_failure_patterns AS
SELECT
    pattern_key,
    lesson_type,
    lesson,
    COUNT(*) as trial_count,
    AVG(CAST(pruned_bpb AS FLOAT)) as avg_pruned_bpb,
    MAX(pattern_count) as max_pattern_count
FROM igla_race_experience
WHERE outcome = 'pruned'
    AND pattern_key IS NOT NULL
GROUP BY pattern_key, lesson_type, lesson
ORDER BY pattern_count DESC, trial_count DESC;

-- ============================================================================
-- VIEW: Active Machines
-- ============================================================================

CREATE OR REPLACE VIEW v_active_machines AS
SELECT
    machine_id,
    COUNT(*) FILTER (WHERE status = 'running') as active_trials,
    COUNT(*) FILTER (WHERE status = 'complete') as completed_trials,
    COUNT(*) FILTER (WHERE status = 'pruned') as pruned_trials,
    MIN(best_bpb) FILTER (WHERE best_bpb IS NOT NULL) as best_bpb,
    MAX(created_at) as last_activity
FROM igla_race_trials
WHERE created_at > NOW() - INTERVAL '1 hour'
GROUP BY machine_id
ORDER BY last_activity DESC;

-- ============================================================================
-- TRIGGERS
-- ============================================================================

-- Auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_trial_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_trial_updated_at
    BEFORE UPDATE ON igla_race_trials
    FOR EACH ROW
    EXECUTE FUNCTION update_trial_updated_at();

-- Auto-extract config fields on insert
CREATE OR REPLACE FUNCTION extract_config_fields()
RETURNS TRIGGER AS $$
BEGIN
    NEW.d_model = (NEW.config->>'d_model')::INT;
    NEW.lr = (NEW.config->>'lr')::FLOAT;
    NEW.wd = (NEW.config->>'wd')::FLOAT;
    NEW.n_gram = (NEW.config->>'n_gram')::INT;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_extract_config_fields
    BEFORE INSERT ON igla_race_trials
    FOR EACH ROW
    EXECUTE FUNCTION extract_config_fields();

-- ============================================================================
-- STORED PROCEDURES
-- ============================================================================

-- Start a new trial (with duplicate prevention)
CREATE OR REPLACE FUNCTION start_trial(
    p_trial_id TEXT,
    p_machine_id TEXT,
    p_worker_id TEXT,
    p_config JSONB
)
RETURNS BOOLEAN AS $$
DECLARE
    v_exists INT;
BEGIN
    -- Check for duplicate running trial with same config
    SELECT COUNT(*) INTO v_exists
    FROM igla_race_trials
    WHERE config::text = p_config::text AND status = 'running';

    IF v_exists > 0 THEN
        -- Duplicate! Return false
        RETURN FALSE;
    END IF;

    INSERT INTO igla_race_trials (
        trial_id, machine_id, worker_id, config, status
    ) VALUES (
        p_trial_id, p_machine_id, p_worker_id, p_config, 'running'
    );

    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

-- Record ASHA rung result
CREATE OR REPLACE FUNCTION record_rung_result(
    p_trial_id TEXT,
    p_rung INT,
    p_bpb FLOAT
)
RETURNS VOID AS $$
BEGIN
    CASE p_rung
        WHEN 1 THEN
            UPDATE igla_race_trials
            SET rung_1_bpb = p_bpb, rung_1_at = NOW()
            WHERE trial_id = p_trial_id;
        WHEN 2 THEN
            UPDATE igla_race_trials
            SET rung_2_bpb = p_bpb, rung_2_at = NOW()
            WHERE trial_id = p_trial_id;
        WHEN 3 THEN
            UPDATE igla_race_trials
            SET rung_3_bpb = p_bpb, rung_3_at = NOW()
            WHERE trial_id = p_trial_id;
        WHEN 4 THEN
            UPDATE igla_race_trials
            SET rung_4_bpb = p_bpb, rung_4_at = NOW()
            WHERE trial_id = p_trial_id;
    END CASE;

    -- Update best_bpb if this is better
    UPDATE igla_race_trials
    SET best_bpb = LEAST(COALESCE(best_bpb, 999.999), p_bpb)
    WHERE trial_id = p_trial_id;
END;
$$ LANGUAGE plpgsql;

-- Complete a trial with experience
CREATE OR REPLACE FUNCTION complete_trial(
    p_trial_id TEXT,
    p_status TEXT,
    p_best_bpb FLOAT,
    p_best_step INT,
    p_pruned_at_step INT DEFAULT NULL,
    p_pruned_reason TEXT DEFAULT NULL,
    p_lesson TEXT DEFAULT NULL,
    p_lesson_type TEXT DEFAULT 'INFO',
    p_pattern_key TEXT DEFAULT NULL
)
RETURNS VOID AS $$
BEGIN
    -- Update trial status
    UPDATE igla_race_trials
    SET
        status = p_status,
        best_bpb = p_best_bpb,
        best_step = p_best_step,
        pruned_at_step = p_pruned_at_step,
        completed_at = NOW()
    WHERE trial_id = p_trial_id;

    -- Insert experience record if provided
    IF p_lesson IS NOT NULL THEN
        INSERT INTO igla_race_experience (
            trial_id, outcome, pruned_at_rung, pruned_bpb, pruned_reason,
            lesson, lesson_type, pattern_key
        ) VALUES (
            p_trial_id,
            CASE
                WHEN p_status = 'pruned' THEN 'pruned'
                WHEN p_status = 'igla_found' THEN 'igla_found'
                ELSE 'complete'
            END,
            p_pruned_at_step,
            p_best_bpb,
            p_pruned_reason,
            p_lesson,
            p_lesson_type,
            p_pattern_key
        );

        -- Update pattern count if pattern_key provided
        IF p_pattern_key IS NOT NULL THEN
            UPDATE igla_race_experience
            SET pattern_count = (
                SELECT COALESCE(COUNT(*) + 1, 1)
                FROM igla_race_experience e2
                WHERE e2.pattern_key = igla_race_experience.pattern_key
            )
            WHERE id = (
                SELECT id
                FROM igla_race_experience
                WHERE trial_id = p_trial_id
                ORDER BY id DESC
                LIMIT 1
            );
        END IF;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Get failure patterns for search space guidance
CREATE OR REPLACE FUNCTION get_failure_patterns(
    p_limit INT DEFAULT 10
)
RETURNS TABLE (
    pattern_key TEXT,
    lesson TEXT,
    lesson_type TEXT,
    count INT,
    avg_bpb FLOAT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        e.pattern_key,
        e.lesson,
        e.lesson_type,
        e.pattern_count,
        AVG(CAST(e.pruned_bpb AS FLOAT))
    FROM igla_race_experience e
    WHERE e.outcome = 'pruned' AND e.pattern_key IS NOT NULL
    GROUP BY e.pattern_key, e.lesson, e.lesson_type, e.pattern_count
    ORDER BY e.pattern_count DESC
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- SAMPLE DATA (for testing)
-- ============================================================================

-- Insert sample trial
INSERT INTO igla_race_trials (trial_id, machine_id, worker_id, config, status)
VALUES (
    'test-trial-001',
    'test-machine-01',
    'worker-0',
    '{"d_model": 128, "lr": 0.005, "wd": 0.01, "n_gram": 5, "activation": "relu", "dropout": 0.0, "warmup": 0, "seed": 42}'::JSONB,
    'complete'
)
ON CONFLICT (trial_id) DO NOTHING;

-- Insert sample experience
INSERT INTO igla_race_experience (trial_id, outcome, pruned_at_rung, pruned_bpb, lesson, lesson_type, pattern_key, pattern_count)
VALUES (
    'test-trial-001',
    'complete',
    NULL,
    2.716,
    'SUCCESS: Good config at d_model=128, lr=0.005',
    'SUCCESS',
    'd_model_128_lr_0.005',
    1
)
ON CONFLICT DO NOTHING;

-- ============================================================================
-- GRANTS (adjust for your user)
-- ============================================================================
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO trios_user;
-- GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO trios_user;
-- GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO trios_user;
