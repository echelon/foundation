ALTER TABLE gateway_service_principals
    DROP CONSTRAINT gateway_service_principals_scopes_check,
    ADD CHECK (cardinality(scopes) BETWEEN 1 AND 5 AND scopes <@
        ARRAY['requests:submit', 'requests:read', 'requests:cancel', 'usage:read', 'exports:read']::text[]);

CREATE TABLE gateway_export_events (
    sid             bigint generated always as identity unique,
    id              text primary key default ('evt_' || replace(gen_random_uuid()::text, '-', '')),
    account_id      text not null,
    project_id      text not null,
    request_id      text not null,
    client_ref      text,
    event_type      text not null check (event_type IN (
        'request.accepted', 'request.dispatched', 'request.succeeded', 'request.canceled',
        'request.failed', 'request.outcome_unconfirmed', 'usage.observed'
    )),
    observation_key text not null default '',
    occurred_at     timestamptz not null,
    data            jsonb not null check (jsonb_typeof(data) = 'object'),
    published_seq   bigint check (published_seq > 0),
    foreign key (account_id, project_id) references gateway_projects (account_id, id),
    unique (request_id, event_type, observation_key),
    unique (account_id, project_id, published_seq)
);

CREATE INDEX gateway_export_pending_idx ON gateway_export_events (sid) WHERE published_seq IS NULL;
CREATE INDEX gateway_export_project_pending_idx ON gateway_export_events (account_id, project_id, sid)
    WHERE published_seq IS NULL;
CREATE INDEX gateway_export_client_events_idx ON gateway_export_events (account_id, project_id, client_ref, published_seq)
    WHERE published_seq IS NOT NULL;

CREATE TABLE gateway_export_heads (
    account_id text not null,
    project_id text not null,
    watermark  bigint not null check (watermark > 0),
    primary key (account_id, project_id),
    foreign key (account_id, project_id) references gateway_projects (account_id, id)
);

CREATE TABLE gateway_export_snapshots (
    id                 text primary key,
    account_id         text not null,
    project_id         text not null,
    requested_by       text not null,
    client_ref         text,
    after_event_id     text,
    after_seq          bigint not null check (after_seq >= 0),
    through_seq        bigint not null check (through_seq >= after_seq),
    pending_at_creation boolean not null,
    created_at         timestamptz not null default now(),
    expires_at         timestamptz not null default (now() + interval '24 hours'),
    foreign key (account_id, project_id) references gateway_projects (account_id, id),
    check (expires_at > created_at)
);
CREATE INDEX gateway_export_snapshots_project_idx ON gateway_export_snapshots (account_id, project_id, id DESC);

CREATE FUNCTION capture_gateway_request_export(request gateway_requests, kind text, happened_at timestamptz)
RETURNS void LANGUAGE sql AS $$
    INSERT INTO gateway_export_events (account_id, project_id, request_id, client_ref, event_type, occurred_at, data)
    VALUES (request.account_id, request.project_id, request.id, request.client_ref, kind, happened_at,
        jsonb_build_object(
            'model', request.model,
            'provider', request.provider,
            'offer_id', request.offer_id,
            'adapter', request.adapter,
            'estimated_usd_nanos', request.estimated_usd_nanos::text
        ))
    ON CONFLICT (request_id, event_type, observation_key) DO NOTHING;
$$;

CREATE FUNCTION capture_gateway_request_export_trigger() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        PERFORM capture_gateway_request_export(NEW, 'request.accepted', NEW.created_at);
        RETURN NULL;
    END IF;
    IF OLD.dispatched_at IS NULL AND NEW.dispatched_at IS NOT NULL THEN
        PERFORM capture_gateway_request_export(NEW, 'request.dispatched', NEW.dispatched_at);
    END IF;
    IF OLD.succeeded_at IS NULL AND NEW.succeeded_at IS NOT NULL THEN
        PERFORM capture_gateway_request_export(NEW, 'request.succeeded', NEW.succeeded_at);
    END IF;
    IF OLD.canceled_at IS NULL AND NEW.canceled_at IS NOT NULL THEN
        PERFORM capture_gateway_request_export(NEW, 'request.canceled', NEW.canceled_at);
    END IF;
    IF OLD.failed_at IS NULL AND NEW.failed_at IS NOT NULL THEN
        PERFORM capture_gateway_request_export(NEW, 'request.failed', NEW.failed_at);
    END IF;
    IF OLD.uncertain_at IS NULL AND NEW.uncertain_at IS NOT NULL THEN
        PERFORM capture_gateway_request_export(NEW, 'request.outcome_unconfirmed', NEW.uncertain_at);
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER gateway_request_export
    AFTER INSERT OR UPDATE OF dispatched_at, succeeded_at, canceled_at, failed_at, uncertain_at ON gateway_requests
    FOR EACH ROW EXECUTE FUNCTION capture_gateway_request_export_trigger();

CREATE FUNCTION capture_gateway_usage_export(observation gateway_usage_observations) RETURNS void LANGUAGE sql AS $$
    INSERT INTO gateway_export_events
        (account_id, project_id, request_id, client_ref, event_type, observation_key, occurred_at, data)
    SELECT r.account_id, r.project_id, r.id, r.client_ref, 'usage.observed',
        observation.attempt_number::text || ':' || observation.source, observation.reported_at,
        jsonb_build_object(
            'attempt_number', observation.attempt_number,
            'source', observation.source,
            'amount_usd_nanos', observation.amount_usd_nanos::text,
            'output_count', observation.output_count,
            'input_tokens', observation.input_tokens,
            'output_tokens', observation.output_tokens,
            'model', r.model,
            'provider', r.provider,
            'upstream_handle', coalesce(p.upstream_handle, a.provider_handle),
            'reported_model', p.reported_model,
            'reported_provider', p.reported_provider
        )
    FROM gateway_requests r
    LEFT JOIN gateway_provider_runs p ON p.request_id = r.id
    LEFT JOIN gateway_provider_attempts a ON a.account_id = r.account_id AND a.request_id = r.id
         AND a.attempt_number = observation.attempt_number
    WHERE r.account_id = observation.account_id AND r.id = observation.request_id
    ON CONFLICT (request_id, event_type, observation_key) DO NOTHING;
$$;

CREATE FUNCTION capture_gateway_usage_export_trigger() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM capture_gateway_usage_export(NEW);
    RETURN NULL;
END;
$$;

CREATE TRIGGER gateway_usage_export AFTER INSERT ON gateway_usage_observations
    FOR EACH ROW EXECUTE FUNCTION capture_gateway_usage_export_trigger();

DO $$
BEGIN
    PERFORM capture_gateway_request_export(r, e.kind, e.at)
    FROM gateway_requests r
    CROSS JOIN LATERAL (VALUES
        ('request.accepted', r.created_at),
        ('request.dispatched', r.dispatched_at),
        ('request.succeeded', r.succeeded_at),
        ('request.canceled', r.canceled_at),
        ('request.failed', r.failed_at),
        ('request.outcome_unconfirmed', r.uncertain_at)
    ) AS e(kind, at)
    WHERE e.at IS NOT NULL
    ORDER BY e.at, r.sid;
    PERFORM capture_gateway_usage_export(o) FROM gateway_usage_observations o ORDER BY o.reported_at, o.request_id;
END;
$$;

CREATE FUNCTION publish_gateway_export_events(batch_size integer, only_project text) RETURNS integer LANGUAGE plpgsql AS $$
DECLARE
    published integer;
BEGIN
    -- Assign cursor order on committed rows. Concurrent admissions never hold this lock.
    IF NOT pg_try_advisory_xact_lock(761293, 1) THEN
        RETURN 0;
    END IF;
    WITH ready AS (
        SELECT id, sid, account_id, project_id
        FROM gateway_export_events
        WHERE published_seq IS NULL AND (only_project IS NULL OR project_id = only_project)
        ORDER BY sid LIMIT batch_size FOR UPDATE SKIP LOCKED
    ), increments AS (
        SELECT account_id, project_id, count(*) AS size FROM ready GROUP BY account_id, project_id
    ), heads AS (
        INSERT INTO gateway_export_heads (account_id, project_id, watermark)
        SELECT account_id, project_id, size FROM increments
        ON CONFLICT (account_id, project_id) DO UPDATE SET watermark = gateway_export_heads.watermark + EXCLUDED.watermark
        RETURNING account_id, project_id, watermark
    ), numbered AS (
        SELECT id, account_id, project_id, row_number() OVER (PARTITION BY account_id, project_id ORDER BY sid) AS row_position
        FROM ready
    )
    UPDATE gateway_export_events e SET published_seq = h.watermark - i.size + n.row_position
    FROM numbered n
    JOIN increments i USING (account_id, project_id)
    JOIN heads h USING (account_id, project_id)
    WHERE e.id = n.id;
    GET DIAGNOSTICS published = ROW_COUNT;
    RETURN published;
END;
$$;
