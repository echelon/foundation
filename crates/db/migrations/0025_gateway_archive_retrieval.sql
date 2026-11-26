ALTER TABLE gateway_archives
    ADD COLUMN retrieve_cold boolean NOT NULL DEFAULT false,
    ADD COLUMN retrieval_attempted_at timestamptz,
    ADD COLUMN restore_check_at timestamptz,
    ADD CONSTRAINT gateway_archives_restore_check_at_check
        CHECK (restore_check_at IS NULL OR (state = 'restore_queued' AND retrieval_attempted_at IS NOT NULL));
