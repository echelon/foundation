ALTER TABLE gateway_archives
    ADD COLUMN retrieval_submissions smallint NOT NULL DEFAULT 0,
    ADD COLUMN retrieval_confirmed boolean NOT NULL DEFAULT false,
    ADD CHECK (retrieval_submissions BETWEEN 0 AND 3),
    ADD CHECK ((retrieval_submissions = 0 AND NOT retrieval_confirmed) OR retrieval_attempted_at IS NOT NULL);
