ALTER TABLE gateway_export_snapshots
    ADD COLUMN automatic_archive boolean NOT NULL DEFAULT false;

ALTER TABLE gateway_archives
    ADD COLUMN automatic boolean NOT NULL DEFAULT false,
    ADD COLUMN automatic_retry_at timestamptz,
    ADD CONSTRAINT gateway_archives_automatic_coverage_check CHECK (
        NOT automatic OR (
            client_ref IS NULL
            AND (stored_at IS NULL OR (NOT has_more AND event_count = through_seq - after_seq))
        )
    ),
    ADD CONSTRAINT gateway_archives_automatic_retry_check CHECK (
        automatic_retry_at IS NULL OR (automatic AND state = 'archive_failed')
    );

CREATE UNIQUE INDEX gateway_automatic_archive_ranges
    ON gateway_archives (account_id, project_id, after_seq)
    WHERE automatic;

CREATE INDEX gateway_automatic_archive_retries
    ON gateway_archives (automatic_retry_at, id)
    WHERE automatic AND state = 'archive_failed';
