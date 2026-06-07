-- bump_strategy_v3 migration
-- Agent F · Wave 21
-- Idempotent: CREATE OR REPLACE
-- v2 left untouched. Zero ALTERs on existing objects.
-- Whitelist adds: account, trainer_bin, w_jepa, w_nca to v2's list.
-- Returns new fingerprint (text) instead of generation (bigint).

CREATE OR REPLACE FUNCTION ssot.bump_strategy_v3(
    p_service_id text,
    p_changes    jsonb
)
RETURNS text
LANGUAGE plpgsql
AS $$
DECLARE
    v_allowed  text[] := ARRAY[
        'optimizer','format','hidden','lr','seed','steps','status',
        'account','trainer_bin','w_jepa','w_nca'
    ];
    v_key      text;
    v_new_seed integer;
    v_valid_seeds integer[] := ARRAY[47, 89, 123, 144, 1597, 2584, 4181, 6765, 10946];
    v_fp       text;
    v_row      ssot.scarab_strategy%ROWTYPE;
BEGIN
    -- 1. Verify service exists
    SELECT * INTO v_row
    FROM ssot.scarab_strategy
    WHERE service_id = p_service_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'bump_strategy_v3: unknown service_id: %', p_service_id;
    END IF;

    -- 2. Validate seed if present
    IF p_changes ? 'seed' THEN
        v_new_seed := (p_changes->>'seed')::integer;
        IF NOT (v_new_seed = ANY(v_valid_seeds)) THEN
            RAISE EXCEPTION 'bump_strategy_v3: seed % not in allowed set {47,89,123,144,1597,2584,4181,6765,10946}', v_new_seed;
        END IF;
    END IF;

    -- 3. Dynamic UPDATE: iterate whitelisted keys present in p_changes
    FOREACH v_key IN ARRAY v_allowed LOOP
        IF p_changes ? v_key THEN
            CASE v_key
                WHEN 'optimizer'    THEN UPDATE ssot.scarab_strategy SET optimizer   = p_changes->>'optimizer'         WHERE service_id = p_service_id;
                WHEN 'format'       THEN UPDATE ssot.scarab_strategy SET format      = p_changes->>'format'            WHERE service_id = p_service_id;
                WHEN 'hidden'       THEN UPDATE ssot.scarab_strategy SET hidden      = (p_changes->>'hidden')::integer  WHERE service_id = p_service_id;
                WHEN 'lr'           THEN UPDATE ssot.scarab_strategy SET lr          = (p_changes->>'lr')::numeric      WHERE service_id = p_service_id;
                WHEN 'seed'         THEN UPDATE ssot.scarab_strategy SET seed        = (p_changes->>'seed')::integer    WHERE service_id = p_service_id;
                WHEN 'steps'        THEN UPDATE ssot.scarab_strategy SET steps       = (p_changes->>'steps')::integer   WHERE service_id = p_service_id;
                WHEN 'status'       THEN UPDATE ssot.scarab_strategy SET status      = p_changes->>'status'            WHERE service_id = p_service_id;
                WHEN 'account'      THEN UPDATE ssot.scarab_strategy SET account     = p_changes->>'account'           WHERE service_id = p_service_id;
                WHEN 'trainer_bin'  THEN UPDATE ssot.scarab_strategy SET trainer_bin = p_changes->>'trainer_bin'       WHERE service_id = p_service_id;
                WHEN 'w_jepa'       THEN UPDATE ssot.scarab_strategy SET w_jepa      = (p_changes->>'w_jepa')::numeric  WHERE service_id = p_service_id;
                WHEN 'w_nca'        THEN UPDATE ssot.scarab_strategy SET w_nca       = (p_changes->>'w_nca')::numeric   WHERE service_id = p_service_id;
                ELSE NULL;
            END CASE;
        END IF;
    END LOOP;

    -- 4. Increment generation + updated_at
    UPDATE ssot.scarab_strategy
    SET generation = generation + 1,
        updated_at = now(),
        updated_by = 'queen-hive'
    WHERE service_id = p_service_id;

    -- 5. Recompute fingerprint from current row state
    SELECT ssot.scarab_fingerprint(optimizer, format, hidden, lr, seed, steps)
    INTO v_fp
    FROM ssot.scarab_strategy
    WHERE service_id = p_service_id;

    -- 6. Record in scarab_command (column is 'command', not 'action')
    INSERT INTO ssot.scarab_command (service_id, command, new_strategy, strategy_fingerprint)
    VALUES (p_service_id, 'bump_v3', p_changes, v_fp);

    -- 7. Return new fingerprint
    RETURN v_fp;
END;
$$;
