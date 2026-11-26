CREATE TABLE gateway_submission_limits (
    account_id text PRIMARY KEY REFERENCES accounts (id) ON DELETE CASCADE,
    requests_per_minute integer NOT NULL DEFAULT 120 CHECK (requests_per_minute BETWEEN 6 AND 60000),
    operator_locked boolean NOT NULL DEFAULT false,
    credit bigint NOT NULL DEFAULT 1200000000 CHECK (credit BETWEEN 0 AND 600000000000),
    refilled_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    last_increase_at timestamptz
);

CREATE TABLE gateway_submission_limit_increases (
    account_id text NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    approved_at timestamptz NOT NULL,
    approved_for_user_id text NOT NULL,
    previous_requests_per_minute integer NOT NULL,
    requests_per_minute integer NOT NULL,
    PRIMARY KEY (account_id, approved_at),
    CHECK (requests_per_minute > previous_requests_per_minute)
);

-- One request costs 60 million credits. Each elapsed microsecond earns RPM credits.
-- Limit elapsed time before multiplying. An idle bucket fills after ten seconds.
CREATE FUNCTION gateway_submission_credit(previous bigint, rpm integer, elapsed interval)
RETURNS bigint LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT least((rpm / 6)::bigint * 60000000,
        previous + floor(greatest(0, least(10, extract(epoch FROM elapsed))) * 1000000)::bigint * rpm)
$$;

-- Called inside admission after idempotency resolution. The debit and request
-- have the same commit outcome. Different organizations lock different rows.
CREATE FUNCTION gateway_consume_submission_credit(p_account_id text)
RETURNS bigint LANGUAGE plpgsql AS $$
DECLARE
    bucket gateway_submission_limits%ROWTYPE;
    observed timestamptz;
    available bigint;
BEGIN
    INSERT INTO gateway_submission_limits (account_id) VALUES (p_account_id)
        ON CONFLICT DO NOTHING;
    SELECT * INTO STRICT bucket FROM gateway_submission_limits
        WHERE account_id = p_account_id FOR UPDATE;
    observed := greatest(clock_timestamp(), bucket.refilled_at);
    available := gateway_submission_credit(bucket.credit, bucket.requests_per_minute, observed - bucket.refilled_at);
    IF available < 60000000 THEN
        RETURN ceil((60000000 - available)::numeric / (bucket.requests_per_minute::bigint * 1000000))::bigint;
    END IF;

    UPDATE gateway_submission_limits SET credit = available - 60000000, refilled_at = observed
        WHERE account_id = p_account_id;
    RETURN 0;
END;
$$;

CREATE FUNCTION gateway_next_submission_limit(rpm integer)
RETURNS integer LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT CASE rpm WHEN 120 THEN 240 WHEN 240 THEN 480 WHEN 480 THEN 600 END
$$;

CREATE FUNCTION gateway_submission_limit(p_account_id text)
RETURNS TABLE (requests_per_minute integer, burst integer, next_self_service_limit integer,
    next_self_service_at timestamptz, operator_managed boolean) LANGUAGE sql STABLE AS $$
    SELECT rpm, rpm / 6, next_rpm,
        CASE WHEN next_rpm IS NOT NULL THEN last_increase_at + interval '24 hours' END,
        managed
    FROM (
        SELECT coalesce(l.requests_per_minute, 120) AS rpm, l.last_increase_at,
            coalesce(l.operator_locked, false) AS managed,
            CASE WHEN NOT coalesce(l.operator_locked, false)
                THEN gateway_next_submission_limit(coalesce(l.requests_per_minute, 120)) END AS next_rpm
        FROM accounts a LEFT JOIN gateway_submission_limits l ON l.account_id = a.id
        WHERE a.id = p_account_id
    ) effective_limit
$$;

-- The expected tier makes concurrent requests safe and successful retries idempotent.
CREATE FUNCTION gateway_increase_submission_limit(
    p_account_id text, p_user_id text, p_expected integer, p_requested integer
) RETURNS text LANGUAGE plpgsql AS $$
DECLARE
    bucket gateway_submission_limits%ROWTYPE;
    observed timestamptz;
    next_rpm integer;
BEGIN
    next_rpm := gateway_next_submission_limit(p_expected);
    IF next_rpm IS NULL OR p_requested IS DISTINCT FROM next_rpm THEN
        RETURN 'invalid_tier';
    END IF;
    INSERT INTO gateway_submission_limits (account_id) VALUES (p_account_id)
        ON CONFLICT DO NOTHING;
    SELECT * INTO STRICT bucket FROM gateway_submission_limits
        WHERE account_id = p_account_id FOR UPDATE;
    IF bucket.operator_locked THEN
        RETURN 'operator_managed';
    END IF;
    IF bucket.requests_per_minute = p_requested AND EXISTS (
        SELECT 1 FROM gateway_submission_limit_increases
        WHERE account_id = p_account_id AND approved_at = bucket.last_increase_at
          AND previous_requests_per_minute = p_expected AND requests_per_minute = p_requested
    ) THEN
        RETURN 'approved';
    END IF;
    IF bucket.requests_per_minute <> p_expected THEN
        RETURN 'stale';
    END IF;
    observed := greatest(clock_timestamp(), bucket.refilled_at);
    IF bucket.last_increase_at + interval '24 hours' > observed THEN
        RETURN 'cooldown';
    END IF;

    UPDATE gateway_submission_limits
        SET credit = gateway_submission_credit(bucket.credit, bucket.requests_per_minute, observed - bucket.refilled_at),
            refilled_at = observed, requests_per_minute = p_requested, last_increase_at = observed
        WHERE account_id = p_account_id;
    INSERT INTO gateway_submission_limit_increases
        VALUES (p_account_id, observed, p_user_id, p_expected, p_requested);
    RETURN 'approved';
END;
$$;
