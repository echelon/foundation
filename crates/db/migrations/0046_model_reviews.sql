CREATE TABLE model_reviews (
    sid bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    model text NOT NULL CHECK (model ~ '^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$'),
    user_id text NOT NULL,
    rating smallint NOT NULL CHECK (rating BETWEEN 1 AND 10),
    review_text text CHECK (octet_length(review_text) BETWEEN 1 AND 1200),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (model, user_id)
);

CREATE INDEX model_reviews_public_page_idx ON model_reviews (model, sid DESC)
    WHERE review_text IS NOT NULL;

CREATE INDEX gateway_requests_recent_model_success_idx
    ON gateway_requests (model, succeeded_at, account_id)
    WHERE succeeded_at IS NOT NULL AND adapter <> 'mock';
