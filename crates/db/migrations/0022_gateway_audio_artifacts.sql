ALTER TABLE gateway_artifacts
    DROP CONSTRAINT gateway_artifacts_check2,
    ADD CONSTRAINT gateway_artifacts_media_size_check CHECK (
        (content_type IN ('image/png', 'audio/wav') AND byte_length BETWEEN 1 AND 2097152)
        OR (content_type = 'video/mp4' AND byte_length BETWEEN 1 AND 8388608)
    );
