ALTER TABLE gateway_provider_runs
    ADD COLUMN publication_retry_step smallint NOT NULL DEFAULT 0,
    ADD CHECK (publication_retry_step BETWEEN 0 AND 4),
    ADD CHECK (publication_retry_step = 0 OR result_observed_at IS NOT NULL);
