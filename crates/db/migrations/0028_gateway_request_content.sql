ALTER TABLE gateway_requests
    ADD COLUMN content_removed_at timestamptz,
    ADD CHECK (content_removed_at IS NULL OR (
        adapter <> 'mock' AND num_nonnulls(succeeded_at, failed_at, canceled_at) = 1
    ));
