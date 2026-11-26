CREATE TABLE gateway_artifacts (
    request_id text NOT NULL REFERENCES gateway_provider_runs(request_id) ON DELETE CASCADE,
    slot smallint NOT NULL CHECK (slot BETWEEN 0 AND 7),
    content_type text NOT NULL CHECK (content_type IN ('image/png')),
    byte_length integer NOT NULL CHECK (byte_length BETWEEN 1 AND 2097152),
    sha256 text NOT NULL CHECK (sha256 ~ '^[a-f0-9]{64}$'),
    payload bytea NOT NULL CHECK (octet_length(payload) = byte_length + 28),
    expires_at timestamptz NOT NULL,
    PRIMARY KEY (request_id, slot)
);

CREATE INDEX gateway_artifacts_expiry_idx ON gateway_artifacts (expires_at);
