CREATE TABLE gateway_connector_completions (
    request_id text PRIMARY KEY REFERENCES gateway_provider_runs (request_id) ON DELETE CASCADE,
    received_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    next_lookup_at timestamptz,
    recovered_from timestamptz,
    reconciled_at timestamptz,
    CHECK ((recovered_from IS NULL) = (reconciled_at IS NULL)),
    CHECK (reconciled_at >= greatest(received_at, recovered_from))
);

CREATE INDEX gateway_connector_completions_pending_idx
    ON gateway_connector_completions (next_lookup_at) WHERE next_lookup_at IS NOT NULL;

CREATE VIEW gateway_provider_callback_receipts AS
    SELECT request_id, received_at, recovered_from, NULL::timestamptz AS reconciled_at
    FROM gateway_fal_callbacks
    UNION ALL
    SELECT request_id, received_at, recovered_from, reconciled_at
    FROM gateway_connector_completions;

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
            SELECT 1 FROM gateway_provider_callback_receipts c WHERE c.request_id = NEW.id AND c.recovered_from IS NOT NULL
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
