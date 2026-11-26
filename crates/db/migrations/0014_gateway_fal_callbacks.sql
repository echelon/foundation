CREATE UNIQUE INDEX gateway_fal_accepted_handle_idx
    ON gateway_provider_runs (binding_digest, upstream_handle) WHERE accepted_at IS NOT NULL;

CREATE TABLE gateway_fal_callbacks (
    request_id text PRIMARY KEY REFERENCES gateway_provider_runs (request_id) ON DELETE CASCADE,
    binding_digest text NOT NULL,
    upstream_handle text NOT NULL,
    body_sha256 text NOT NULL CHECK (body_sha256 ~ '^[a-f0-9]{64}$'),
    received_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    recovered_from timestamptz,
    UNIQUE (binding_digest, upstream_handle),
    CHECK (recovered_from IS NULL OR recovered_from <= received_at)
);

CREATE OR REPLACE FUNCTION enqueue_gateway_webhook() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    event_id text := 'whd_' || substr(NEW.id, 5);
    terminal_time timestamptz := coalesce(NEW.succeeded_at, NEW.canceled_at, NEW.failed_at);
    terminal_status text := CASE
        WHEN NEW.succeeded_at IS NOT NULL THEN 'succeeded'
        WHEN NEW.canceled_at IS NOT NULL THEN 'canceled'
        ELSE 'failed'
    END;
    event_type text := 'request.' || terminal_status;
    terminal_sequence integer := CASE
        WHEN NEW.dispatched_at IS NULL THEN 2
        WHEN EXISTS (
            SELECT 1 FROM gateway_fal_callbacks c WHERE c.request_id = NEW.id AND c.recovered_from IS NOT NULL
        ) THEN 5
        ELSE 3
    END;
BEGIN
    INSERT INTO gateway_webhook_deliveries (id, request_id, event_type, occurred_at, payload)
    VALUES (event_id, NEW.id, event_type, terminal_time, jsonb_build_object(
        'id', event_id,
        'type', event_type,
        'schema_version', 1,
        'occurred_at', to_char(terminal_time AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"'),
        'sequence', terminal_sequence,
        'data', jsonb_build_object(
            'request_id', NEW.id,
            'project_id', NEW.project_id,
            'status', terminal_status,
            'fetch_url', '/v1/requests/' || NEW.id
        )
    )::text);
    RETURN NULL;
END;
$$;
