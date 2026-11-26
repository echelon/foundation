ALTER TABLE gateway_requests
    DROP CONSTRAINT gateway_requests_account_id_provider_credential_scope_id_a_fkey,
    ADD COLUMN provider_credential_binding_slot text;

ALTER TABLE gateway_provider_credentials
    DROP CONSTRAINT gateway_provider_credentials_pkey,
    DROP CONSTRAINT gateway_provider_credentials_adapter_check,
    ADD COLUMN binding_slot text NOT NULL DEFAULT 'primary',
    ADD CHECK (adapter IN ('openrouter_chat', 'kinovi_queue', 'fal_queue')),
    ADD CHECK (binding_slot = 'primary' OR (binding_slot = 'secondary' AND adapter = 'fal_queue')),
    ADD PRIMARY KEY (account_id, scope_id, adapter, binding_slot, version);

UPDATE gateway_requests
SET provider_credential_binding_slot = 'primary'
WHERE provider_credential_version IS NOT NULL;

ALTER TABLE gateway_requests
    ADD CHECK ((provider_credential_binding_slot IS NULL) = (provider_credential_version IS NULL)),
    ADD CONSTRAINT gateway_requests_credential_pin_fkey
        FOREIGN KEY (account_id, provider_credential_scope_id, adapter,
                     provider_credential_binding_slot, provider_credential_version)
        REFERENCES gateway_provider_credentials (account_id, scope_id, adapter, binding_slot, version);

DROP INDEX gateway_requests_credential_idx;
CREATE INDEX gateway_requests_credential_idx
    ON gateway_requests (account_id, provider_credential_scope_id, adapter,
                         provider_credential_binding_slot, provider_credential_version)
    WHERE provider_credential_version IS NOT NULL;
