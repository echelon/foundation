BEGIN READ ONLY;
SET LOCAL statement_timeout = '2000ms';

-- Sample the oldest currently claimable submissions for the deployed bindings.
-- Exclude scheduled delays, disabled routes, and requests cooling down upstream.
WITH binding AS (
    SELECT ARRAY(SELECT jsonb_array_elements_text(:'local_bindings'::jsonb)) AS local_chat,
           ARRAY(SELECT jsonb_array_elements_text(:'connector_bindings'::jsonb)) AS connector
), eligible AS (
    SELECT r.dispatch_after
    FROM gateway_requests r JOIN gateway_provider_runs p ON p.request_id = r.id
    CROSS JOIN binding b
    WHERE ((r.adapter = 'openai_compatible_chat' AND p.binding_digest = ANY(b.local_chat))
           OR (r.adapter = 'connector_queue' AND p.binding_digest = ANY(b.connector)))
      AND r.dispatched_at IS NULL AND r.failed_at IS NULL AND r.canceled_at IS NULL
      AND r.dispatch_after <= now() AND r.dispatch_after > now() - interval '15 minutes'
      AND r.created_at > now() - interval '15 minutes'
      AND p.request_payload IS NOT NULL
      AND NOT EXISTS (
          SELECT 1 FROM gateway_dispatch_controls d
          WHERE d.account_id = r.account_id AND (d.pause_all OR r.provider = ANY(d.paused_providers))
      )
      AND (r.adapter <> 'connector_queue' OR NOT EXISTS (
          SELECT 1 FROM gateway_queue_cooldowns c
          WHERE c.account_id = r.account_id AND c.project_id = r.project_id
            AND c.binding_digest = p.binding_digest AND c.upstream_model = p.upstream_model
            AND c.available_after > now()
      ))
    ORDER BY r.dispatch_after, r.sid
    LIMIT 256
), connections AS (
    SELECT greatest(0, current_setting('max_connections')::integer
        - current_setting('superuser_reserved_connections')::integer
        - coalesce(current_setting('reserved_connections', true)::integer, 0)
        - :'reserve_connections'::integer - count(*)) AS free
    FROM pg_stat_activity WHERE backend_type = 'client backend'
)
SELECT json_build_object(
    'oldest_ready_age_seconds', coalesce(floor(extract(epoch FROM now() - min(dispatch_after)))::integer, 0),
    'eligible_sample', count(*),
    'additional_connections', (SELECT free FROM connections)
) FROM eligible;

COMMIT;
