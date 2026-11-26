-- Account and project capacity checks read only unresolved accepted requests.
CREATE INDEX gateway_requests_active_scope_idx
    ON gateway_requests (account_id, project_id, provider)
    WHERE num_nonnulls(succeeded_at, failed_at, canceled_at) = 0;
