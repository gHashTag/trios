-- IGLA Race Database Schema
-- 3 tables: trials, experience, leaderboard view

-- Table 1: igla_race_trials
CREATE TABLE IF NOT EXISTS igla_race_trials (
    trial_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    machine_id VARCHAR(100) NOT NULL,
    worker_id INTEGER NOT NULL,
    config JSONB NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    
    -- ASHA rungs (3^k Trinity: 1k -> 3k -> 9k -> 27k)
    rung_1000_step INTEGER,
    rung_1000_bpb FLOAT,
    rung_3000_step INTEGER,
    rung_3000_bpb FLOAT,
    rung_9000_step INTEGER,
    rung_9000_bpb FLOAT,
    rung_27000_step INTEGER,
    rung_27000_bpb FLOAT,
    
    final_step INTEGER,
    final_bpb FLOAT,
    
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    pruned_at TIMESTAMPTZ,
    duration_seconds FLOAT,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_trials_status ON igla_race_trials(status);
CREATE INDEX idx_trials_machine ON igla_race_trials(machine_id);
CREATE INDEX idx_trials_bpb ON igla_race_trials(final_bpb) WHERE final_bpb IS NOT NULL;

-- Table 2: igla_race_experience (Failure Memory)
CREATE TABLE IF NOT EXISTS igla_race_experience (
    experience_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trial_id UUID NOT NULL REFERENCES igla_race_trials(trial_id) ON DELETE CASCADE,
    
    outcome VARCHAR(20) NOT NULL,
    pruned_at_rung INTEGER NOT NULL,
    bpb_at_pruned FLOAT NOT NULL,
    
    lesson TEXT NOT NULL,
    lesson_type VARCHAR(20) NOT NULL,
    
    pattern_count INTEGER DEFAULT 1,
    confidence FLOAT DEFAULT 1.0,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_experience_trial ON igla_race_experience(trial_id);
CREATE INDEX idx_experience_lesson ON igla_race_experience(lesson_type, pattern_count DESC);

-- Table 3: Leaderboard VIEW
CREATE OR REPLACE VIEW igla_race_leaderboard AS
SELECT
    t.trial_id,
    t.machine_id,
    t.config,
    t.status,
    t.final_bpb,
    t.final_step,
    t.duration_seconds,
    t.started_at,
    t.completed_at,
    CASE
        WHEN t.rung_27000_bpb IS NOT NULL THEN 27000
        WHEN t.rung_9000_bpb IS NOT NULL THEN 9000
        WHEN t.rung_3000_bpb IS NOT NULL THEN 3000
        WHEN t.rung_1000_bpb IS NOT NULL THEN 1000
        ELSE 0
    END as best_rung,
    (SELECT lesson FROM igla_race_experience e 
     WHERE e.trial_id = t.trial_id 
     ORDER BY e.pattern_count DESC LIMIT 1) as lesson,
    RANK() OVER (ORDER BY t.final_bpb ASC NULLS LAST) as bpb_rank
FROM igla_race_trials t
WHERE t.status IN ('completed', 'pruned') AND t.final_bpb IS NOT NULL;

-- Function: Auto-update updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_trials_updated_at
    BEFORE UPDATE ON igla_race_trials
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function: Check duplicate running trial
CREATE OR REPLACE FUNCTION check_trial_running(p_machine_id VARCHAR, p_config JSONB)
RETURNS BOOLEAN AS $$
DECLARE
    trial_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO trial_count
    FROM igla_race_trials
    WHERE machine_id = p_machine_id AND config = p_config
      AND status IN ('pending', 'running');
    RETURN trial_count > 0;
END;
$$ LANGUAGE plpgsql;

-- Function: Get top lessons
CREATE OR REPLACE FUNCTION get_top_lessons(p_limit INTEGER DEFAULT 10)
RETURNS TABLE (lesson TEXT, lesson_type VARCHAR, pattern_count INTEGER) AS $$
BEGIN
    RETURN QUERY
    SELECT lesson, lesson_type, pattern_count
    FROM igla_race_experience
    ORDER BY pattern_count DESC, confidence DESC
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;
