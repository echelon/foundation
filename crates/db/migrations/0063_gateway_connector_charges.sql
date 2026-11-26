ALTER TABLE gateway_usage_observations
    ADD COLUMN charge_unit text,
    ADD COLUMN charge_quantity numeric,
    DROP CONSTRAINT gateway_usage_observations_source_check,
    DROP CONSTRAINT gateway_usage_observations_output_count_check,
    ADD CONSTRAINT gateway_usage_observations_source_check CHECK (source IN (
        'mock_provider', 'openrouter_report', 'openrouter_stream_partial', 'fal_result',
        'local_chat_report', 'local_chat_stream_partial', 'kinovi_result', 'connector_result', 'connector_charge'
    )),
    ADD CONSTRAINT gateway_usage_observations_output_count_check CHECK (
        CASE WHEN source IN ('openrouter_stream_partial', 'local_chat_stream_partial', 'connector_charge')
             THEN output_count = 0 ELSE output_count > 0 END
    ),
    ADD CONSTRAINT gateway_usage_observations_charge_check CHECK (
        CASE WHEN source = 'connector_charge' THEN
            charge_unit IS NOT NULL AND charge_unit ~ '^[a-z][a-z0-9_]{0,31}$'
            AND charge_quantity IS NOT NULL AND charge_quantity >= 0
            AND charge_quantity < 1000000000000000 AND scale(charge_quantity) <= 9
            AND num_nonnulls(amount_usd_nanos, input_tokens, output_tokens) = 0
        ELSE charge_unit IS NULL AND charge_quantity IS NULL END
    );

CREATE OR REPLACE FUNCTION capture_gateway_usage_export(observation gateway_usage_observations) RETURNS void LANGUAGE sql AS $$
    INSERT INTO gateway_export_events
        (account_id, project_id, request_id, client_ref, event_type, observation_key, occurred_at, data)
    SELECT r.account_id, r.project_id, r.id, r.client_ref, 'usage.observed',
        observation.attempt_number::text || ':' || observation.source, observation.reported_at,
        jsonb_build_object(
            'attempt_number', observation.attempt_number,
            'source', observation.source,
            'amount_usd_nanos', observation.amount_usd_nanos::text,
            'output_count', observation.output_count,
            'input_tokens', observation.input_tokens,
            'output_tokens', observation.output_tokens,
            'model', r.model,
            'provider', r.provider,
            'upstream_handle', coalesce(p.upstream_handle, a.provider_handle),
            'reported_model', p.reported_model,
            'reported_provider', p.reported_provider
        ) || CASE WHEN observation.charge_unit IS NOT NULL THEN jsonb_build_object(
            'provider_charge', jsonb_build_object('unit', observation.charge_unit, 'quantity', observation.charge_quantity::text)
        ) ELSE '{}'::jsonb END
    FROM gateway_requests r
    LEFT JOIN gateway_provider_runs p ON p.request_id = r.id
    LEFT JOIN gateway_provider_attempts a ON a.account_id = r.account_id AND a.request_id = r.id
         AND a.attempt_number = observation.attempt_number
    WHERE r.account_id = observation.account_id AND r.id = observation.request_id
    ON CONFLICT (request_id, event_type, observation_key) DO NOTHING;
$$;
