ALTER TABLE gateway_archives
    ADD COLUMN storage_target text NOT NULL DEFAULT 'files',
    ADD CONSTRAINT gateway_archives_storage_target_check
        CHECK (storage_target = 'files' OR storage_target ~ '^s3_[a-f0-9]{64}$');
