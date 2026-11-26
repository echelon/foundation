ALTER TABLE gateway_provider_runs
    ADD COLUMN recheck_requested_at timestamptz,
    ADD COLUMN recheck_unconfirmed_at timestamptz,
    ADD CHECK (
        (recheck_requested_at IS NULL AND recheck_unconfirmed_at IS NULL)
        OR (recheck_requested_at IS NOT NULL AND recheck_unconfirmed_at IS NOT NULL
            AND accepted_at IS NOT NULL AND upstream_handle IS NOT NULL
            AND recheck_requested_at >= recheck_unconfirmed_at)
    );
