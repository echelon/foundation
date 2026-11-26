ALTER TABLE gateway_archives
    ADD COLUMN retired_at timestamptz,
    ADD COLUMN retirement_check_at timestamptz,
    ADD COLUMN retirement_error_code text,
    ADD UNIQUE (account_id, project_id, id),
    ADD CONSTRAINT gateway_archives_retirement_check CHECK (
        retired_at IS NULL OR (
            automatic AND stored_at IS NOT NULL
            AND retirement_check_at IS NULL AND retirement_error_code IS NULL
        )
    );

CREATE INDEX gateway_archives_retirement_ready
    ON gateway_archives (stored_at, id)
    WHERE automatic AND stored_at IS NOT NULL AND retired_at IS NULL;

ALTER TABLE gateway_export_events
    ALTER COLUMN data DROP NOT NULL,
    ADD COLUMN archive_id text,
    ADD FOREIGN KEY (account_id, project_id, archive_id) REFERENCES gateway_archives (account_id, project_id, id),
    ADD CONSTRAINT gateway_export_events_archive_payload_check CHECK (
        (data IS NULL) = (archive_id IS NOT NULL)
        AND (archive_id IS NULL OR published_seq IS NOT NULL)
    );
