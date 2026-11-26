CREATE TABLE gateway_artifact_objects (
    object_key text PRIMARY KEY CHECK (object_key ~ '^[0-9a-f]{32}$'),
    request_id text NOT NULL REFERENCES gateway_provider_runs (request_id),
    target text NOT NULL CHECK (target ~ '^s3_[0-9a-f]{64}$'),
    cleanup_at timestamptz DEFAULT (statement_timestamp() + interval '5 minutes'),
    deleting boolean NOT NULL DEFAULT false CHECK (NOT deleting OR cleanup_at IS NOT NULL),
    UNIQUE (request_id, object_key)
);

CREATE INDEX gateway_artifact_objects_cleanup_idx ON gateway_artifact_objects (target, cleanup_at)
    WHERE cleanup_at IS NOT NULL;

ALTER TABLE gateway_artifacts
    DROP CONSTRAINT gateway_artifacts_storage_check,
    ADD COLUMN bucket_key text UNIQUE,
    ADD CONSTRAINT gateway_artifacts_bucket_object_fk
        FOREIGN KEY (request_id, bucket_key) REFERENCES gateway_artifact_objects (request_id, object_key),
    ADD CONSTRAINT gateway_artifacts_storage_check
        CHECK (num_nonnulls(payload, object_key, bucket_key, archive_sha256) = 1);

CREATE FUNCTION release_gateway_artifact_object() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        IF OLD.bucket_key IS NOT DISTINCT FROM NEW.bucket_key THEN
            RETURN NULL;
        END IF;
    END IF;
    UPDATE gateway_artifact_objects SET cleanup_at = statement_timestamp()
    WHERE object_key = OLD.bucket_key AND cleanup_at IS NULL;
    RETURN NULL;
END;
$$;

CREATE TRIGGER gateway_artifacts_release_bucket_object
    AFTER UPDATE OF bucket_key OR DELETE ON gateway_artifacts
    FOR EACH ROW WHEN (OLD.bucket_key IS NOT NULL)
    EXECUTE FUNCTION release_gateway_artifact_object();
