CREATE TABLE gateway_client_budgets (
    account_id text NOT NULL,
    project_id text NOT NULL,
    client_ref text COLLATE "C" NOT NULL CHECK (client_ref ~ '^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$'),
    revision integer NOT NULL CHECK (revision > 0),
    limit_usd_nanos bigint CHECK (limit_usd_nanos BETWEEN 0 AND 1000000000000000),
    PRIMARY KEY (account_id, project_id, client_ref),
    FOREIGN KEY (account_id, project_id) REFERENCES gateway_projects (account_id, id)
);

CREATE INDEX gateway_requests_client_month_idx
    ON gateway_requests (account_id, project_id, client_ref, created_at) WHERE client_ref IS NOT NULL;

ALTER TABLE gateway_requests
    ADD COLUMN client_budget_revision integer,
    ADD COLUMN client_budget_limit_usd_nanos bigint,
    ADD CHECK (
        (client_budget_revision IS NULL AND client_budget_limit_usd_nanos IS NULL)
        OR (client_budget_revision IS NOT NULL AND client_budget_revision > 0
            AND client_ref IS NOT NULL AND client_budget_limit_usd_nanos IS NOT NULL
            AND client_budget_limit_usd_nanos BETWEEN 0 AND 1000000000000000)
    );
