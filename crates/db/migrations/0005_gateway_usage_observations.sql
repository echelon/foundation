CREATE TABLE gateway_usage_observations (
    account_id         text not null,
    request_id         text not null,
    attempt_number     smallint not null check (attempt_number > 0),
    source             text not null check (source = 'mock_provider'),
    amount_usd_nanos   bigint not null check (amount_usd_nanos >= 0),
    output_count       smallint not null check (output_count > 0),
    reported_at        timestamptz not null,
    primary key (account_id, request_id, attempt_number, source)
);

INSERT INTO gateway_usage_observations
    (account_id, request_id, attempt_number, source, amount_usd_nanos, output_count, reported_at)
SELECT a.account_id, a.request_id, a.attempt_number, 'mock_provider', a.reported_usd_nanos,
       coalesce((r.quote #>> '{input,workload,count}')::smallint, 1), a.completed_at
FROM gateway_provider_attempts a
JOIN gateway_requests r ON r.account_id = a.account_id AND r.id = a.request_id
WHERE a.completed_at IS NOT NULL AND a.canceled_at IS NULL;

CREATE INDEX gateway_requests_usage_window_idx ON gateway_requests (account_id, created_at);
