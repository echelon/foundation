ALTER TABLE gateway_requests
    DROP CONSTRAINT gateway_requests_adapter_check,
    ADD CONSTRAINT gateway_requests_adapter_check
        CHECK (adapter IN ('mock', 'openrouter_chat', 'fal_queue', 'openai_compatible_chat', 'kinovi_queue'));

ALTER TABLE gateway_usage_observations
    DROP CONSTRAINT gateway_usage_observations_source_check,
    ADD CONSTRAINT gateway_usage_observations_source_check CHECK (source IN (
        'mock_provider', 'openrouter_report', 'openrouter_stream_partial', 'fal_result',
        'local_chat_report', 'local_chat_stream_partial', 'kinovi_result'
    ));
