ALTER TABLE gateway_requests
    ADD COLUMN succeeded_at timestamptz,
    ADD COLUMN canceled_at timestamptz;

UPDATE gateway_requests r
SET succeeded_at = a.completed_at
FROM gateway_provider_attempts a
WHERE a.account_id = r.account_id AND a.request_id = r.id
  AND a.attempt_number = 1 AND a.completed_at IS NOT NULL;

ALTER TABLE gateway_requests ADD CONSTRAINT gateway_requests_single_terminal_state
    CHECK (succeeded_at IS NULL OR (canceled_at IS NULL AND dispatched_at IS NOT NULL));

ALTER TABLE gateway_provider_attempts
    ADD COLUMN canceled_at timestamptz,
    ADD CONSTRAINT gateway_provider_attempts_single_terminal_state
        CHECK (canceled_at IS NULL OR completed_at IS NULL);

DROP INDEX gateway_requests_pending_idx;
CREATE INDEX gateway_requests_pending_idx ON gateway_requests (dispatch_after, sid)
    WHERE dispatched_at IS NULL AND canceled_at IS NULL;

DROP INDEX gateway_provider_attempts_pending_idx;
CREATE INDEX gateway_provider_attempts_pending_idx ON gateway_provider_attempts (complete_after, request_id)
    WHERE completed_at IS NULL AND canceled_at IS NULL;
