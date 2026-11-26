ALTER TABLE gateway_content_history
    DROP CONSTRAINT gateway_chat_history_target_check,
    ADD CONSTRAINT gateway_content_history_target_check
        CHECK (target ~ '^(s3_)?[0-9a-f]{64}$');
