-- Stamp attribution at capture so published exports and sealed archive bytes stay stable.
CREATE FUNCTION stamp_gateway_export_app_ref() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    reference text;
BEGIN
    SELECT r.app_ref INTO reference
    FROM gateway_requests r
    WHERE r.id = NEW.request_id AND r.account_id = NEW.account_id AND r.project_id = NEW.project_id
      AND r.client_ref IS NOT DISTINCT FROM NEW.client_ref AND r.app_ref IS NOT NULL;
    IF FOUND THEN
        NEW.data := NEW.data || jsonb_build_object('app_ref', reference);
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER gateway_export_app_ref
    BEFORE INSERT ON gateway_export_events
    FOR EACH ROW EXECUTE FUNCTION stamp_gateway_export_app_ref();
