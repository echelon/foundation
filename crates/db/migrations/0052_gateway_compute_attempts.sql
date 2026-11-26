ALTER TABLE gateway_provider_runs
    ADD COLUMN compute_attempts jsonb NOT NULL DEFAULT '[]'::jsonb,
    ADD CONSTRAINT gateway_provider_runs_compute_attempts_check
        CHECK (jsonb_typeof(compute_attempts) = 'array' AND jsonb_array_length(compute_attempts) <= 4);
