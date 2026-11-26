CREATE TABLE gateway_supplier_settlements (
    request_id text NOT NULL REFERENCES gateway_requests (id),
    revision integer NOT NULL CHECK (revision > 0),
    amount_usd_nanos bigint NOT NULL CHECK (amount_usd_nanos >= 0),
    delta_usd_nanos bigint NOT NULL,
    evidence_ref text NOT NULL CHECK (evidence_ref ~ '^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$'),
    idempotency_digest text NOT NULL CHECK (idempotency_digest ~ '^[a-f0-9]{64}$'),
    recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (request_id, revision),
    UNIQUE (request_id, idempotency_digest)
);

CREATE FUNCTION sequence_gateway_supplier_settlement() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    previous_revision integer;
    previous_amount bigint;
BEGIN
    PERFORM id FROM gateway_requests WHERE id = NEW.request_id AND adapter <> 'mock'
        AND dispatched_at IS NOT NULL
        AND num_nonnulls(succeeded_at, failed_at, canceled_at, uncertain_at) <> 0
        FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'supplier settlement requires a dispatched provider request with an outcome'
            USING ERRCODE = '23514';
    END IF;
    SELECT revision, amount_usd_nanos INTO previous_revision, previous_amount
        FROM gateway_supplier_settlements
        WHERE request_id = NEW.request_id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision <> coalesce(previous_revision, 0)::bigint + 1 OR NEW.amount_usd_nanos < 0 THEN
        RAISE EXCEPTION 'invalid supplier settlement revision or amount' USING ERRCODE = '23514';
    END IF;
    NEW.delta_usd_nanos := NEW.amount_usd_nanos - coalesce(previous_amount, 0);
    RETURN NEW;
END;
$$;

CREATE TRIGGER gateway_supplier_settlement_sequence
    BEFORE INSERT ON gateway_supplier_settlements
    FOR EACH ROW EXECUTE FUNCTION sequence_gateway_supplier_settlement();

CREATE FUNCTION protect_gateway_supplier_settlement() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'supplier settlements are append-only' USING ERRCODE = '55000';
END;
$$;

CREATE TRIGGER gateway_supplier_settlement_immutable
    BEFORE UPDATE OR DELETE ON gateway_supplier_settlements
    FOR EACH ROW EXECUTE FUNCTION protect_gateway_supplier_settlement();

ALTER TABLE gateway_export_events
    DROP CONSTRAINT gateway_export_events_event_type_check,
    ADD CHECK (event_type IN (
        'request.accepted', 'request.dispatched', 'request.succeeded', 'request.canceled',
        'request.failed', 'request.outcome_unconfirmed', 'usage.observed', 'usage.supplier_settled'
    ));

CREATE FUNCTION capture_gateway_supplier_settlement() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO gateway_export_events (account_id, project_id, request_id, client_ref, event_type,
                                       observation_key, occurred_at, data)
    SELECT r.account_id, r.project_id, r.id, r.client_ref, 'usage.supplier_settled',
        NEW.revision::text, NEW.recorded_at, jsonb_build_object(
            'revision', NEW.revision,
            'amount_usd_nanos', NEW.amount_usd_nanos::text,
            'delta_usd_nanos', NEW.delta_usd_nanos::text,
            'provider', r.provider,
            'model', r.model
        )
    FROM gateway_requests r WHERE r.id = NEW.request_id;
    RETURN NULL;
END;
$$;

CREATE TRIGGER gateway_supplier_settlement_export
    AFTER INSERT ON gateway_supplier_settlements
    FOR EACH ROW EXECUTE FUNCTION capture_gateway_supplier_settlement();
