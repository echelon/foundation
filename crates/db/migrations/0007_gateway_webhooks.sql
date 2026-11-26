CREATE TABLE gateway_webhook_subscriptions (
    id                 text primary key,
    account_id         text not null,
    project_id         text not null,
    name               text not null check (length(name) between 1 and 80),
    destination_id     text not null,
    destination_url    text not null,
    client_ref         text check (client_ref ~ '^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$'),
    signing_version    integer not null default 1 check (signing_version > 0),
    signing_rotated_at timestamptz,
    created_at         timestamptz not null default now(),
    revoked_at         timestamptz,
    unique (account_id, project_id, id),
    foreign key (account_id, project_id) references gateway_projects (account_id, id)
);

ALTER TABLE gateway_requests ADD COLUMN webhook_subscription_id text,
    ADD FOREIGN KEY (account_id, project_id, webhook_subscription_id)
        REFERENCES gateway_webhook_subscriptions (account_id, project_id, id);

CREATE TABLE gateway_webhook_deliveries (
    id               text primary key,
    request_id       text not null unique references gateway_requests (id),
    event_type       text not null check (event_type IN ('request.succeeded', 'request.canceled')),
    occurred_at      timestamptz not null,
    payload          text not null,
    state            text not null default 'pending' check (state IN ('pending', 'sending', 'delivered', 'dead')),
    available_at     timestamptz not null default clock_timestamp(),
    deadline_at      timestamptz not null default (clock_timestamp() + interval '15 minutes'),
    attempt_count    integer not null default 0 check (attempt_count >= 0),
    attempt_baseline integer not null default 0 check (attempt_baseline between 0 and attempt_count),
    finished_at      timestamptz,
    last_error       text,
    check ((state IN ('delivered', 'dead')) = (finished_at IS NOT NULL))
);

CREATE INDEX gateway_webhook_due_idx ON gateway_webhook_deliveries (available_at, id)
    WHERE state IN ('pending', 'sending');

CREATE TABLE gateway_webhook_attempts (
    delivery_id text not null references gateway_webhook_deliveries (id),
    number      integer not null check (number > 0),
    started_at  timestamptz not null default clock_timestamp(),
    finished_at timestamptz,
    outcome     text check (outcome IN (
        'delivered', 'http_error', 'network_error', 'lease_expired', 'destination_unavailable'
    )),
    http_status smallint check (http_status between 100 and 999),
    primary key (delivery_id, number),
    check ((finished_at IS NULL) = (outcome IS NULL))
);

CREATE FUNCTION enqueue_gateway_webhook() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    event_id text := 'whd_' || substr(NEW.id, 5);
    terminal_time timestamptz := coalesce(NEW.succeeded_at, NEW.canceled_at);
    terminal_status text := CASE WHEN NEW.succeeded_at IS NOT NULL THEN 'succeeded' ELSE 'canceled' END;
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

CREATE TRIGGER gateway_request_terminal_webhook
    AFTER UPDATE OF succeeded_at, canceled_at ON gateway_requests
    FOR EACH ROW WHEN (
        NEW.webhook_subscription_id IS NOT NULL
        AND OLD.succeeded_at IS NULL AND OLD.canceled_at IS NULL
        AND (NEW.succeeded_at IS NOT NULL OR NEW.canceled_at IS NOT NULL)
    ) EXECUTE FUNCTION enqueue_gateway_webhook();
