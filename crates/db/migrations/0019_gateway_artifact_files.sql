CREATE TABLE gateway_artifact_namespace (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    id uuid NOT NULL DEFAULT gen_random_uuid()
);

INSERT INTO gateway_artifact_namespace DEFAULT VALUES;

ALTER TABLE gateway_artifacts
    ALTER COLUMN payload DROP NOT NULL,
    ADD COLUMN object_key text,
    ADD CHECK (num_nonnulls(payload, object_key) = 1),
    ADD CHECK (object_key IS NULL OR object_key ~ '^[a-f0-9]{32}$'),
    ADD UNIQUE (object_key);
