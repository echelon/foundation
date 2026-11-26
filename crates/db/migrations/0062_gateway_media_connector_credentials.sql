ALTER TABLE gateway_provider_credentials
    DROP CONSTRAINT gateway_provider_credentials_adapter_check,
    DROP CONSTRAINT gateway_provider_credentials_key_size_check,
    DROP CONSTRAINT gateway_provider_credentials_slot_check,
    ADD CONSTRAINT gateway_provider_credentials_adapter_check
        CHECK (adapter IN ('openrouter_chat', 'kinovi_queue', 'fal_queue', 'openai_compatible_chat', 'connector_queue')),
    ADD CONSTRAINT gateway_provider_credentials_key_size_check
        CHECK (octet_length(encrypted_key) BETWEEN 37 AND
            CASE WHEN adapter IN ('openrouter_chat', 'openai_compatible_chat', 'connector_queue') THEN 156 ELSE 92 END),
    ADD CONSTRAINT gateway_provider_credentials_slot_check
        CHECK (
            (adapter IN ('openai_compatible_chat', 'connector_queue') AND binding_slot ~ '^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$')
            OR (adapter NOT IN ('openai_compatible_chat', 'connector_queue') AND
                (binding_slot = 'primary' OR (binding_slot = 'secondary' AND adapter = 'fal_queue')))
        );
