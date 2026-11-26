CREATE TABLE gateway_chat_history (
    request_id text PRIMARY KEY REFERENCES gateway_provider_runs (request_id),
    target text NOT NULL CHECK (target ~ '^[0-9a-f]{64}$'),
    input bytea CHECK (octet_length(input) <= 16384),
    archive_at timestamptz,
    expires_at timestamptz,
    digest text CHECK (digest ~ '^[0-9a-f]{64}$'),
    retry_at timestamptz,
    purged_at timestamptz,
    CHECK ((archive_at IS NULL) = (expires_at IS NULL)),
    CHECK (archive_at < expires_at),
    CHECK (digest IS NULL OR input IS NULL)
);

CREATE INDEX gateway_chat_history_archive_idx ON gateway_chat_history (target, archive_at)
    WHERE digest IS NULL AND purged_at IS NULL;
CREATE INDEX gateway_chat_history_expire_idx ON gateway_chat_history (expires_at)
    WHERE purged_at IS NULL;
CREATE INDEX gateway_chat_history_cleanup_idx ON gateway_chat_history (target, request_id)
    WHERE purged_at IS NULL AND input IS NULL;

CREATE FUNCTION start_gateway_chat_history_clock() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    finished_at timestamptz := coalesce(NEW.succeeded_at, NEW.failed_at, NEW.canceled_at, NEW.uncertain_at);
BEGIN
    UPDATE gateway_chat_history SET
        archive_at = finished_at + interval '720 hours',
        expires_at = finished_at + interval '9480 hours'
    WHERE request_id = NEW.id AND archive_at IS NULL;
    RETURN NULL;
END;
$$;

CREATE TRIGGER gateway_chat_history_clock
    AFTER UPDATE OF succeeded_at, failed_at, canceled_at, uncertain_at ON gateway_requests
    FOR EACH ROW WHEN (coalesce(NEW.succeeded_at, NEW.failed_at, NEW.canceled_at, NEW.uncertain_at) IS NOT NULL)
    EXECUTE FUNCTION start_gateway_chat_history_clock();

CREATE FUNCTION remove_gateway_chat_history_input() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    UPDATE gateway_chat_history SET input = NULL, retry_at = NULL WHERE request_id = NEW.id;
    RETURN NULL;
END;
$$;

CREATE TRIGGER gateway_chat_history_removal
    AFTER UPDATE OF content_removed_at ON gateway_requests
    FOR EACH ROW WHEN (OLD.content_removed_at IS NULL AND NEW.content_removed_at IS NOT NULL)
    EXECUTE FUNCTION remove_gateway_chat_history_input();
