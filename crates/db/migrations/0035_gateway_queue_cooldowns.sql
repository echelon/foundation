CREATE TABLE gateway_queue_cooldowns (
    account_id text NOT NULL,
    project_id text NOT NULL,
    binding_digest text NOT NULL CHECK (binding_digest ~ '^[a-f0-9]{64}$'),
    upstream_model text NOT NULL,
    available_after timestamptz NOT NULL,
    PRIMARY KEY (account_id, project_id, binding_digest, upstream_model),
    FOREIGN KEY (account_id, project_id) REFERENCES gateway_projects (account_id, id) ON DELETE CASCADE
);

CREATE INDEX gateway_queue_cooldowns_expiry ON gateway_queue_cooldowns (available_after);
