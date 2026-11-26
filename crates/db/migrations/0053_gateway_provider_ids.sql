ALTER TABLE gateway_dispatch_controls
    DROP CONSTRAINT gateway_dispatch_controls_paused_providers_check,
    ADD CONSTRAINT gateway_dispatch_provider_ids CHECK (
        cardinality(paused_providers) <= 64
        AND array_position(paused_providers, NULL) IS NULL
        AND (cardinality(paused_providers) = 0
          OR array_to_string(paused_providers, ',') ~ '^[a-z][a-z0-9_-]{0,63}(,[a-z][a-z0-9_-]{0,63})*$')
    );
