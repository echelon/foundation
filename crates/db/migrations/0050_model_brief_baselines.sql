ALTER TABLE model_reviews
    ADD COLUMN text_revision bigint NOT NULL DEFAULT 1 CHECK (text_revision > 0);

ALTER TABLE model_review_briefs
    ADD COLUMN baseline jsonb CHECK (jsonb_typeof(baseline) = 'object'),
    ADD COLUMN community jsonb CHECK (jsonb_typeof(community) = 'array'),
    ADD COLUMN review_sample jsonb NOT NULL DEFAULT '[]' CHECK (jsonb_typeof(review_sample) = 'array');

-- Existing briefs remain readable and get their research baseline on the next refresh.
UPDATE model_review_briefs SET refresh_after = least(refresh_after, now());

DROP INDEX model_review_briefs_pending_idx;
CREATE INDEX model_review_briefs_pending_idx ON model_review_briefs (refresh_after, model)
    WHERE baseline IS NULL OR revision > published_revision;
