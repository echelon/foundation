-- Successful runs keep their result deadline until scoped content removal.
-- Include rows already purged by an old gateway so their consumers receive a
-- removal event after the upgrade.
CREATE INDEX gateway_provider_result_expiry_idx
    ON gateway_provider_runs (result_expires_at, request_id)
    WHERE result_expires_at IS NOT NULL;

DROP INDEX gateway_provider_payload_expiry_idx;
