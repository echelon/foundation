CREATE TABLE gateway_provider_credentials (
    account_id text NOT NULL,
    project_id text NOT NULL,
    adapter text NOT NULL CHECK (adapter = 'openrouter_chat'),
    version integer NOT NULL CHECK (version > 0),
    binding_digest text NOT NULL CHECK (binding_digest ~ '^[a-f0-9]{64}$'),
    encrypted_key bytea CHECK (octet_length(encrypted_key) BETWEEN 37 AND 92),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    retired_at timestamptz,
    revoked_at timestamptz,
    PRIMARY KEY (account_id, project_id, adapter, version),
    FOREIGN KEY (account_id, project_id) REFERENCES gateway_projects (account_id, id),
    CHECK (revoked_at IS NULL OR encrypted_key IS NULL),
    CHECK (encrypted_key IS NOT NULL OR retired_at IS NOT NULL OR revoked_at IS NOT NULL)
);

ALTER TABLE gateway_requests
    ADD COLUMN provider_credential_version integer,
    ADD FOREIGN KEY (account_id, project_id, adapter, provider_credential_version)
        REFERENCES gateway_provider_credentials (account_id, project_id, adapter, version);

CREATE INDEX gateway_requests_credential_idx
    ON gateway_requests (account_id, project_id, adapter, provider_credential_version)
    WHERE provider_credential_version IS NOT NULL;

CREATE INDEX gateway_provider_retired_keys_idx
    ON gateway_provider_credentials (retired_at)
    WHERE encrypted_key IS NOT NULL AND retired_at IS NOT NULL;
