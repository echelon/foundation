CREATE TABLE model_review_briefs (
    model text PRIMARY KEY CHECK (model ~ '^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$'),
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    published_revision bigint NOT NULL DEFAULT 0 CHECK (published_revision BETWEEN 0 AND revision),
    refresh_after timestamptz NOT NULL DEFAULT now(),
    refreshed_at timestamptz,
    rating_count bigint NOT NULL DEFAULT 0 CHECK (rating_count >= 0),
    rating_total bigint NOT NULL DEFAULT 0 CHECK (rating_total >= 0),
    written_reviews_sampled integer NOT NULL DEFAULT 0 CHECK (written_reviews_sampled BETWEEN 0 AND 16),
    strengths text[] NOT NULL DEFAULT '{}' CHECK (cardinality(strengths) <= 2),
    weaknesses text[] NOT NULL DEFAULT '{}' CHECK (cardinality(weaknesses) <= 2)
);

CREATE INDEX model_review_briefs_pending_idx ON model_review_briefs (refresh_after, model)
    WHERE revision > published_revision;

INSERT INTO model_review_briefs (model)
SELECT DISTINCT r.model FROM model_reviews r JOIN users u ON u.id = r.user_id
WHERE u.deleted_at IS NULL;
