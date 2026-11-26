CREATE SEQUENCE gateway_queue_turn_seq;

CREATE TABLE gateway_queue_turns (
    project_id text NOT NULL REFERENCES gateway_projects (id) ON DELETE CASCADE,
    lane       text NOT NULL CHECK (lane IN ('provider_submit', 'provider_poll', 'mock_submit')),
    turn      bigint NOT NULL DEFAULT nextval('gateway_queue_turn_seq'),
    PRIMARY KEY (project_id, lane)
);

ALTER SEQUENCE gateway_queue_turn_seq OWNED BY gateway_queue_turns.turn;
