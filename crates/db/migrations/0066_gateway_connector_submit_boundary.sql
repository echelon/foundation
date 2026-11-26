-- Preserve the last authorized call across connector retries so a verified
-- callback can suspend the next submission while it is queued or being claimed.
ALTER TABLE gateway_requests
    ADD COLUMN connector_submit_started_at timestamptz;
