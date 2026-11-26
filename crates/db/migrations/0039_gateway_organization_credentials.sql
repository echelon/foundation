ALTER TABLE gateway_requests
    DROP CONSTRAINT gateway_requests_account_id_project_id_adapter_provider_cr_fkey,
    ADD COLUMN provider_credential_scope_id text;

ALTER TABLE gateway_provider_credentials
    DROP CONSTRAINT gateway_provider_credentials_pkey,
    DROP CONSTRAINT gateway_provider_credentials_check1,
    ALTER COLUMN project_id DROP NOT NULL,
    ADD COLUMN scope_id text GENERATED ALWAYS AS (coalesce(project_id, account_id)) STORED,
    ADD COLUMN inherits_organization boolean NOT NULL DEFAULT false,
    ADD PRIMARY KEY (account_id, scope_id, adapter, version),
    ADD FOREIGN KEY (account_id) REFERENCES accounts (id),
    ADD CHECK (project_id IS NULL OR project_id <> account_id),
    ADD CHECK (encrypted_key IS NOT NULL OR retired_at IS NOT NULL OR revoked_at IS NOT NULL OR inherits_organization),
    ADD CHECK (NOT inherits_organization OR (
        project_id IS NOT NULL AND encrypted_key IS NULL AND revoked_at IS NULL
    ));

UPDATE gateway_requests
SET provider_credential_scope_id = project_id
WHERE provider_credential_version IS NOT NULL;

ALTER TABLE gateway_requests
    ADD CHECK ((provider_credential_scope_id IS NULL) = (provider_credential_version IS NULL)),
    ADD CHECK (provider_credential_scope_id IS NULL OR provider_credential_scope_id IN (account_id, project_id)),
    ADD FOREIGN KEY (account_id, provider_credential_scope_id, adapter, provider_credential_version)
        REFERENCES gateway_provider_credentials (account_id, scope_id, adapter, version);

DROP INDEX gateway_requests_credential_idx;
CREATE INDEX gateway_requests_credential_idx
    ON gateway_requests (account_id, provider_credential_scope_id, adapter, provider_credential_version)
    WHERE provider_credential_version IS NOT NULL;
