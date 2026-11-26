CREATE TABLE gateway_dispatch_controls (
    account_id text PRIMARY KEY REFERENCES accounts (id),
    revision integer NOT NULL CHECK (revision > 0),
    pause_all boolean NOT NULL,
    paused_providers text[] NOT NULL CHECK (
        cardinality(paused_providers) <= 4
        AND array_position(paused_providers, NULL) IS NULL
        AND paused_providers <@ ARRAY['fal', 'kinovi', 'openrouter', 'self_hosted']::text[]
    )
);
