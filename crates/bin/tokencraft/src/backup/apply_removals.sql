-- Only run inside the private, newly restored staging database.
CREATE TEMP TABLE content_checkpoint (
    request_id text PRIMARY KEY CHECK (request_id ~ '^req_[0-9A-Za-z]{26}$'),
    account_id text NOT NULL,
    project_id text NOT NULL,
    created_at timestamptz NOT NULL,
    removed_at timestamptz NOT NULL,
    succeeded_at timestamptz,
    failed_at timestamptz,
    canceled_at timestamptz,
    event_id text NOT NULL UNIQUE CHECK (event_id ~ '^evt_[a-f0-9]{32}$'),
    CHECK (num_nonnulls(succeeded_at, failed_at, canceled_at) = 1)
);
\copy content_checkpoint FROM 'removals.csv' CSV

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM content_checkpoint c
        JOIN gateway_requests r ON r.id = c.request_id
        LEFT JOIN gateway_export_events e ON e.request_id = r.id
            AND e.event_type = 'request.content_removed' AND e.observation_key = ''
        WHERE (r.account_id, r.project_id, r.created_at) <> (c.account_id, c.project_id, c.created_at)
           OR r.adapter = 'mock'
           OR num_nonnulls(r.succeeded_at, r.failed_at, r.canceled_at, r.uncertain_at) <> 1
           OR (r.uncertain_at IS NULL AND
               (r.succeeded_at, r.failed_at, r.canceled_at)
                   IS DISTINCT FROM (c.succeeded_at, c.failed_at, c.canceled_at))
           OR (r.content_removed_at IS NOT NULL AND
               (r.content_removed_at, e.id) IS DISTINCT FROM (c.removed_at, c.event_id))
    ) THEN
        RAISE EXCEPTION 'Content removal checkpoint does not match restored request identity or state';
    END IF;
END
$$;

CREATE TEMP TABLE recovered_removals AS
SELECT c.*
FROM content_checkpoint c
JOIN gateway_requests r ON r.id = c.request_id
WHERE r.content_removed_at IS NULL;

UPDATE gateway_provider_runs p
SET request_payload = NULL, response_payload = NULL, result_expires_at = NULL,
    lease_until = NULL, poll_after = NULL, poll_deadline = NULL,
    recheck_requested_at = NULL, recheck_unconfirmed_at = NULL
FROM content_checkpoint c WHERE p.request_id = c.request_id;

DELETE FROM gateway_chat_deltas d USING content_checkpoint c WHERE d.request_id = c.request_id;
DELETE FROM gateway_artifacts a USING content_checkpoint c WHERE a.request_id = c.request_id;

-- An old snapshot may predate resolution of an uncertain request. Restore its
-- outcome without scheduling a historical webhook or another lifecycle event.
ALTER TABLE gateway_requests DISABLE TRIGGER gateway_request_terminal_webhook;
ALTER TABLE gateway_requests DISABLE TRIGGER gateway_request_export;
UPDATE gateway_requests r
SET content_removed_at = c.removed_at,
    succeeded_at = CASE WHEN r.uncertain_at IS NOT NULL THEN c.succeeded_at ELSE r.succeeded_at END,
    failed_at = CASE WHEN r.uncertain_at IS NOT NULL THEN c.failed_at ELSE r.failed_at END,
    canceled_at = CASE WHEN r.uncertain_at IS NOT NULL THEN c.canceled_at ELSE r.canceled_at END,
    uncertain_at = NULL
FROM recovered_removals c
WHERE r.id = c.request_id;
ALTER TABLE gateway_requests ENABLE TRIGGER gateway_request_export;
ALTER TABLE gateway_requests ENABLE TRIGGER gateway_request_terminal_webhook;

-- The existing trigger creates the removal event with a new cursor. Preserve
-- the original event ID for imports that already observed this removal.
UPDATE gateway_export_events e
SET id = c.event_id
FROM recovered_removals c
WHERE e.request_id = c.request_id AND e.event_type = 'request.content_removed'
  AND e.observation_key = '' AND e.published_seq IS NULL;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM recovered_removals c
        LEFT JOIN gateway_export_events e ON e.request_id = c.request_id
            AND e.event_type = 'request.content_removed' AND e.observation_key = ''
        WHERE e.id IS DISTINCT FROM c.event_id
    ) THEN
        RAISE EXCEPTION 'Could not preserve the original content removal event';
    END IF;
END
$$;

\copy (SELECT request_id FROM content_checkpoint) TO 'applied-requests'
