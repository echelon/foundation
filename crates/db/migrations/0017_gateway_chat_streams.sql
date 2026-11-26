ALTER TABLE gateway_provider_runs
    ADD COLUMN stream_started_at timestamptz,
    ADD COLUMN stream_detached_at timestamptz;

CREATE TABLE gateway_chat_deltas (
    request_id text NOT NULL REFERENCES gateway_provider_runs (request_id) ON DELETE CASCADE,
    ordinal smallint NOT NULL CHECK (ordinal BETWEEN 1 AND 512),
    payload bytea NOT NULL CHECK (octet_length(payload) BETWEEN 29 AND 32800),
    stored_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (request_id, ordinal)
);

CREATE INDEX gateway_chat_deltas_age_idx ON gateway_chat_deltas (stored_at);

ALTER TABLE gateway_usage_observations
    DROP CONSTRAINT gateway_usage_observations_source_check,
    DROP CONSTRAINT gateway_usage_observations_output_count_check,
    ADD CHECK (source IN ('mock_provider', 'openrouter_report', 'openrouter_stream_partial', 'fal_result')),
    ADD CHECK (
        (source = 'openrouter_stream_partial' AND output_count = 0) OR
        (source <> 'openrouter_stream_partial' AND output_count > 0)
    );
