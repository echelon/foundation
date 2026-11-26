ALTER TABLE gateway_requests
    ADD COLUMN trace_id text,
    ADD COLUMN admission_span_id text,
    ADD COLUMN caller_trace_id text,
    ADD COLUMN caller_span_id text,
    ADD CHECK (
        (trace_id IS NULL AND admission_span_id IS NULL)
        OR (trace_id IS NOT NULL AND trace_id ~ '^[0-9a-f]{32}$' AND trace_id <> repeat('0', 32)
            AND admission_span_id IS NOT NULL AND admission_span_id ~ '^[0-9a-f]{16}$'
            AND admission_span_id <> repeat('0', 16))
    ),
    ADD CHECK (
        (caller_trace_id IS NULL AND caller_span_id IS NULL)
        OR (trace_id IS NOT NULL AND caller_trace_id IS NOT NULL AND caller_trace_id ~ '^[0-9a-f]{32}$'
            AND caller_trace_id <> repeat('0', 32)
            AND caller_span_id IS NOT NULL AND caller_span_id ~ '^[0-9a-f]{16}$'
            AND caller_span_id <> repeat('0', 16))
    );

CREATE INDEX gateway_requests_trace_id_idx ON gateway_requests (account_id, project_id, trace_id, sid DESC)
    WHERE trace_id IS NOT NULL;
CREATE INDEX gateway_requests_caller_trace_id_idx ON gateway_requests (account_id, project_id, caller_trace_id, sid DESC)
    WHERE caller_trace_id IS NOT NULL;
