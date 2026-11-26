ALTER TABLE gateway_provider_runs
    ADD COLUMN result_observed_at timestamptz,
    ADD CHECK (result_observed_at IS NULL OR (accepted_at IS NOT NULL AND upstream_handle IS NOT NULL));
