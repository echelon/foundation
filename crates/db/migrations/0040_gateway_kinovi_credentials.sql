ALTER TABLE gateway_provider_credentials
    DROP CONSTRAINT gateway_provider_credentials_adapter_check,
    DROP CONSTRAINT gateway_provider_credentials_encrypted_key_check,
    ADD CHECK (adapter IN ('openrouter_chat', 'kinovi_queue')),
    -- The vault adds a 12-byte nonce and 16-byte authentication tag.
    ADD CHECK (octet_length(encrypted_key) BETWEEN 37 AND
        CASE adapter WHEN 'openrouter_chat' THEN 156 ELSE 92 END);
