ALTER TABLE gateway_requests
    DROP CONSTRAINT gateway_requests_adapter_check,
    ALTER COLUMN mock_latency_ms DROP NOT NULL,
    ADD COLUMN failed_at timestamptz,
    ADD COLUMN uncertain_at timestamptz,
    ADD CHECK (adapter IN ('mock', 'openrouter_chat')),
    ADD CHECK ((adapter = 'mock') = (mock_latency_ms IS NOT NULL)),
    ADD CHECK (num_nonnulls(succeeded_at, canceled_at, failed_at, uncertain_at) <= 1);

CREATE TABLE gateway_provider_runs (
    request_id       text primary key references gateway_requests (id),
    binding_digest   text not null check (length(binding_digest) = 64),
    upstream_model   text not null check (length(upstream_model) between 1 and 160),
    request_payload  bytea,
    response_payload bytea,
    result_expires_at timestamptz,
    lease_until      timestamptz,
    upstream_handle  text,
    reported_model   text,
    reported_provider text,
    finish_reason    text,
    error_code       text,
    check (request_payload IS NULL OR octet_length(request_payload) <= 16384),
    check (response_payload IS NULL OR (
        result_expires_at IS NOT NULL AND octet_length(response_payload) <= 262144
    ))
);

CREATE INDEX gateway_provider_pending_idx ON gateway_requests (dispatch_after, sid)
    WHERE adapter <> 'mock' AND dispatched_at IS NULL AND canceled_at IS NULL;
CREATE INDEX gateway_provider_lease_idx ON gateway_provider_runs (lease_until) WHERE lease_until IS NOT NULL;
CREATE INDEX gateway_provider_payload_expiry_idx ON gateway_provider_runs (result_expires_at)
    WHERE response_payload IS NOT NULL;

ALTER TABLE gateway_usage_observations
    DROP CONSTRAINT gateway_usage_observations_source_check,
    ALTER COLUMN amount_usd_nanos DROP NOT NULL,
    ADD COLUMN input_tokens integer check (input_tokens >= 0),
    ADD COLUMN output_tokens integer check (output_tokens >= 0),
    ADD CHECK (source IN ('mock_provider', 'openrouter_report')),
    ADD CHECK (source <> 'mock_provider' OR amount_usd_nanos IS NOT NULL);

ALTER TABLE gateway_webhook_deliveries
    DROP CONSTRAINT gateway_webhook_deliveries_event_type_check,
    ADD CHECK (event_type IN ('request.succeeded', 'request.canceled', 'request.failed'));

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
BEGIN
    INSERT INTO gateway_webhook_deliveries (id, request_id, event_type, occurred_at, payload)
    VALUES (event_id, NEW.id, event_type, terminal_time, jsonb_build_object(
        'id', event_id,
        'type', event_type,
        'schema_version', 1,
        'occurred_at', to_char(terminal_time AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"'),
        'sequence', CASE WHEN NEW.dispatched_at IS NULL THEN 2 ELSE 3 END,
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

DROP TRIGGER gateway_request_terminal_webhook ON gateway_requests;
CREATE TRIGGER gateway_request_terminal_webhook
    AFTER UPDATE OF succeeded_at, canceled_at, failed_at ON gateway_requests
    FOR EACH ROW WHEN (
        NEW.webhook_subscription_id IS NOT NULL
        AND OLD.succeeded_at IS NULL AND OLD.canceled_at IS NULL AND OLD.failed_at IS NULL
        AND (NEW.succeeded_at IS NOT NULL OR NEW.canceled_at IS NOT NULL OR NEW.failed_at IS NOT NULL)
    ) EXECUTE FUNCTION enqueue_gateway_webhook();
