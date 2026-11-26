CREATE TABLE gateway_requests (
    sid                 bigint generated always as identity unique,
    id                  text primary key,
    account_id          text not null,
    actor_id            text not null,
    idempotency_digest  text not null check (length(idempotency_digest) = 64),
    intent_digest       text not null check (length(intent_digest) = 64),
    client_ref          text check (client_ref ~ '^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$'),
    model               text not null,
    offer_id            text not null,
    provider            text not null,
    quote               jsonb not null check (jsonb_typeof(quote) = 'object'),
    adapter             text not null check (adapter = 'mock'),
    mock_latency_ms     integer not null check (mock_latency_ms between 100 and 30000),
    estimated_usd_nanos bigint not null check (estimated_usd_nanos >= 0),
    created_at          timestamptz not null default now(),
    dispatch_after      timestamptz not null default now(),
    dispatched_at       timestamptz,
    unique (account_id, actor_id, idempotency_digest),
    unique (account_id, id)
);

CREATE INDEX gateway_requests_history_idx ON gateway_requests (account_id, sid desc);
CREATE INDEX gateway_requests_client_idx ON gateway_requests (account_id, client_ref, sid desc);
CREATE INDEX gateway_requests_pending_idx ON gateway_requests (dispatch_after, sid) WHERE dispatched_at IS NULL;

CREATE TABLE gateway_provider_attempts (
    account_id          text not null,
    request_id          text not null,
    attempt_number      smallint not null check (attempt_number > 0),
    provider_handle     text not null,
    submitted_at        timestamptz not null,
    complete_after      timestamptz not null,
    completed_at        timestamptz,
    reported_usd_nanos  bigint check (reported_usd_nanos >= 0),
    primary key (account_id, request_id, attempt_number),
    foreign key (account_id, request_id) references gateway_requests (account_id, id),
    check ((completed_at is null) = (reported_usd_nanos is null)),
    check (complete_after >= submitted_at),
    check (completed_at is null or completed_at >= complete_after)
);

CREATE INDEX gateway_provider_attempts_pending_idx
    ON gateway_provider_attempts (complete_after, request_id) WHERE completed_at IS NULL;
