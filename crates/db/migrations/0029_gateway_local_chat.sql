ALTER TABLE gateway_requests
    DROP CONSTRAINT gateway_requests_adapter_check,
    ADD CONSTRAINT gateway_requests_adapter_check
        CHECK (adapter IN ('mock', 'openrouter_chat', 'fal_queue', 'openai_compatible_chat'));

ALTER TABLE gateway_usage_observations
    DROP CONSTRAINT gateway_usage_observations_source_check,
    DROP CONSTRAINT gateway_usage_observations_check1,
    ADD CONSTRAINT gateway_usage_observations_source_check CHECK (source IN (
        'mock_provider', 'openrouter_report', 'openrouter_stream_partial', 'fal_result',
        'local_chat_report', 'local_chat_stream_partial'
    )),
    ADD CONSTRAINT gateway_usage_observations_output_count_check CHECK (
        CASE WHEN source IN ('openrouter_stream_partial', 'local_chat_stream_partial')
             THEN output_count = 0 ELSE output_count > 0 END
    );
