ALTER TABLE gateway_requests
    ADD COLUMN budget_reserved_usd_nanos bigint
        CHECK (budget_reserved_usd_nanos >= 0);

CREATE INDEX gateway_requests_account_month_idx ON gateway_requests (account_id, created_at);
