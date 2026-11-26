ALTER TABLE gateway_requests
    DROP CONSTRAINT gateway_requests_adapter_check,
    ADD CHECK (adapter IN ('mock', 'openrouter_chat', 'fal_queue'));

ALTER TABLE gateway_provider_runs
    ADD COLUMN accepted_at timestamptz,
    ADD COLUMN poll_after timestamptz,
    ADD COLUMN poll_deadline timestamptz,
    ADD COLUMN queue_state text CHECK (queue_state IN ('in_queue', 'in_progress', 'completed')),
    ADD CHECK (poll_after IS NULL OR (
        accepted_at IS NOT NULL AND upstream_handle IS NOT NULL AND poll_deadline IS NOT NULL
    ));

CREATE INDEX gateway_provider_poll_idx ON gateway_provider_runs (poll_after, request_id)
    WHERE poll_after IS NOT NULL;

ALTER TABLE gateway_usage_observations
    DROP CONSTRAINT gateway_usage_observations_source_check,
    ADD CHECK (source IN ('mock_provider', 'openrouter_report', 'fal_result'));
