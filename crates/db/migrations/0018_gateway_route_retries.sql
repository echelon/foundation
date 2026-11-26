ALTER TABLE gateway_provider_runs
    ADD COLUMN route_retries jsonb NOT NULL DEFAULT '[]'::jsonb,
    ADD CHECK (jsonb_typeof(route_retries) = 'array' AND jsonb_array_length(route_retries) <= 3);
