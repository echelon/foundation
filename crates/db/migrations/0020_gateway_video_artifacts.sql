ALTER TABLE gateway_artifacts
    DROP CONSTRAINT gateway_artifacts_content_type_check,
    DROP CONSTRAINT gateway_artifacts_byte_length_check,
    ADD CHECK (
        (content_type = 'image/png' AND byte_length BETWEEN 1 AND 2097152)
        OR (content_type = 'video/mp4' AND byte_length BETWEEN 1 AND 8388608)
    );
