-- Match the lock order of request writers before extending the export constraint
-- and installing the trigger. The previous constraint already covers existing rows.
LOCK TABLE gateway_requests IN SHARE ROW EXCLUSIVE MODE;

ALTER TABLE gateway_export_events
    DROP CONSTRAINT gateway_export_events_event_type_check,
    ADD CONSTRAINT gateway_export_events_event_type_check CHECK (event_type IN (
        'request.accepted', 'request.dispatched', 'request.succeeded', 'request.canceled',
        'request.failed', 'request.outcome_unconfirmed', 'request.content_removed',
        'usage.observed', 'usage.supplier_settled'
    )) NOT VALID;

CREATE FUNCTION capture_gateway_content_removal() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO gateway_export_events
        (account_id, project_id, request_id, client_ref, event_type, occurred_at, data)
    VALUES
        (NEW.account_id, NEW.project_id, NEW.id, NEW.client_ref,
         'request.content_removed', NEW.content_removed_at, '{}'::jsonb)
    ON CONFLICT (request_id, event_type, observation_key) DO NOTHING;
    RETURN NULL;
END;
$$;

CREATE TRIGGER gateway_content_removal_export
    AFTER UPDATE OF content_removed_at ON gateway_requests
    FOR EACH ROW
    WHEN (OLD.content_removed_at IS NULL AND NEW.content_removed_at IS NOT NULL)
    EXECUTE FUNCTION capture_gateway_content_removal();

-- The publisher appends these events after existing cursors. Their occurrence
-- timestamps come from the original removals.
INSERT INTO gateway_export_events
    (account_id, project_id, request_id, client_ref, event_type, occurred_at, data)
SELECT account_id, project_id, id, client_ref, 'request.content_removed', content_removed_at, '{}'::jsonb
FROM gateway_requests
WHERE content_removed_at IS NOT NULL
ORDER BY content_removed_at, id
ON CONFLICT (request_id, event_type, observation_key) DO NOTHING;
