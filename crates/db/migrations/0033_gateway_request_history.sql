ALTER TABLE gateway_requests
    ADD COLUMN app_ref TEXT CHECK (app_ref ~ '^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$');

CREATE INDEX gateway_requests_app_history_idx
    ON gateway_requests (account_id, project_id, app_ref, sid DESC)
    WHERE app_ref IS NOT NULL;
