CREATE TABLE gateway_content_restores (
    request_id text NOT NULL REFERENCES gateway_content_history (request_id) ON DELETE CASCADE,
    slot smallint NOT NULL CHECK (slot BETWEEN -1 AND 7),
    checksum text NOT NULL CHECK (checksum ~ '^[a-f0-9]{64}$'),
    byte_limit integer NOT NULL CHECK (byte_limit BETWEEN 1 AND 8388636),
    requested_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    check_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    attempted boolean NOT NULL DEFAULT false,
    state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'ready', 'failed')),
    PRIMARY KEY (request_id, slot)
);

CREATE INDEX gateway_content_restores_due_idx ON gateway_content_restores (check_at)
    WHERE state = 'pending';

CREATE OR REPLACE FUNCTION remove_gateway_chat_history_input() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM gateway_content_restores WHERE request_id = NEW.id;
    UPDATE gateway_content_history SET input = NULL, retry_at = NULL WHERE request_id = NEW.id;
    RETURN NULL;
END;
$$;
