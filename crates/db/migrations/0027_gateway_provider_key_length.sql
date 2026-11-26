-- Encrypted keys contain a 12-byte nonce and a 16-byte authentication tag.
ALTER TABLE gateway_provider_credentials
    DROP CONSTRAINT gateway_provider_credentials_encrypted_key_check,
    ADD CONSTRAINT gateway_provider_credentials_encrypted_key_check
        CHECK (octet_length(encrypted_key) BETWEEN 37 AND 156);
