ALTER TABLE gateway_provider_runs
    ADD COLUMN connector_recovery_enabled boolean NOT NULL DEFAULT false;

-- A lookup lease can exist without a callback. The first callback can also
-- arrive after the worker has already verified the handle independently.
ALTER TABLE gateway_connector_completions
    ALTER COLUMN received_at DROP NOT NULL,
    DROP CONSTRAINT gateway_connector_completions_check1,
    ADD CONSTRAINT gateway_connector_recovery_chronology CHECK (reconciled_at >= recovered_from);

CREATE INDEX gateway_connector_unconfirmed_recent_idx
    ON gateway_requests (uncertain_at, id)
    WHERE adapter = 'connector_queue' AND uncertain_at IS NOT NULL
      AND num_nonnulls(succeeded_at, failed_at, canceled_at, content_removed_at) = 0;
