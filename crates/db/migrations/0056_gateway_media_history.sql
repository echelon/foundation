ALTER TABLE gateway_chat_history RENAME TO gateway_content_history;

CREATE OR REPLACE FUNCTION start_gateway_chat_history_clock() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    finished_at timestamptz := coalesce(NEW.succeeded_at, NEW.failed_at, NEW.canceled_at, NEW.uncertain_at);
BEGIN
    UPDATE gateway_content_history SET
        archive_at = finished_at + interval '720 hours',
        expires_at = finished_at + interval '9480 hours'
    WHERE request_id = NEW.id AND archive_at IS NULL;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION remove_gateway_chat_history_input() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    UPDATE gateway_content_history SET input = NULL, retry_at = NULL WHERE request_id = NEW.id;
    RETURN NULL;
END;
$$;

-- A media slot keeps one storage location throughout its lifetime. Preserve the
-- original media metadata so retrieval can verify the plaintext independently.
ALTER TABLE gateway_artifacts
    DROP CONSTRAINT gateway_artifacts_check1,
    ADD COLUMN archive_sha256 text CHECK (archive_sha256 ~ '^[a-f0-9]{64}$'),
    ADD CONSTRAINT gateway_artifacts_storage_check
        CHECK (num_nonnulls(payload, object_key, archive_sha256) = 1);
