CREATE TABLE gateway_projects (
    id         text primary key,
    account_id text not null,
    name       text not null check (length(name) between 1 and 80),
    is_default boolean not null default false,
    created_at timestamptz not null default now(),
    unique (account_id, id)
);

CREATE UNIQUE INDEX gateway_projects_default_idx ON gateway_projects (account_id) WHERE is_default;

INSERT INTO gateway_projects (id, account_id, name, is_default)
SELECT 'proj_' || substr(id, 6), id, 'Default', true FROM accounts;

ALTER TABLE gateway_requests ADD COLUMN project_id text;

UPDATE gateway_requests r SET project_id = p.id
FROM gateway_projects p WHERE p.account_id = r.account_id AND p.is_default;

ALTER TABLE gateway_requests
    ALTER COLUMN project_id SET NOT NULL,
    DROP CONSTRAINT gateway_requests_account_id_actor_id_idempotency_digest_key,
    ADD UNIQUE (account_id, project_id, actor_id, idempotency_digest),
    ADD FOREIGN KEY (account_id, project_id) REFERENCES gateway_projects (account_id, id);

DROP INDEX gateway_requests_history_idx;
DROP INDEX gateway_requests_client_idx;
DROP INDEX gateway_requests_usage_window_idx;
CREATE INDEX gateway_requests_history_idx ON gateway_requests (account_id, project_id, sid desc);
CREATE INDEX gateway_requests_client_idx ON gateway_requests (account_id, project_id, client_ref, sid desc);
CREATE INDEX gateway_requests_usage_window_idx ON gateway_requests (account_id, project_id, created_at);

CREATE TABLE gateway_project_grants (
    account_id    text not null,
    project_id    text not null,
    membership_id text not null,
    role          text not null check (role IN ('developer', 'billing_viewer', 'client_viewer')),
    client_ref    text check (client_ref ~ '^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$'),
    primary key (account_id, project_id, membership_id),
    foreign key (account_id, project_id) references gateway_projects (account_id, id),
    check ((role = 'client_viewer') = (client_ref IS NOT NULL))
);

CREATE TABLE gateway_service_principals (
    id         text primary key,
    account_id text not null,
    project_id text not null,
    name       text not null check (length(name) between 1 and 80),
    scopes     text[] not null check (cardinality(scopes) BETWEEN 1 AND 4 AND scopes <@
        ARRAY['requests:submit', 'requests:read', 'requests:cancel', 'usage:read']::text[]),
    client_ref text check (client_ref ~ '^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$'),
    created_at timestamptz not null default now(),
    revoked_at timestamptz,
    unique (account_id, project_id, id),
    foreign key (account_id, project_id) references gateway_projects (account_id, id)
);

CREATE TABLE gateway_project_keys (
    id           text primary key,
    account_id   text not null,
    project_id   text not null,
    principal_id text not null,
    key_prefix   text not null,
    key_hash     text not null unique check (length(key_hash) = 64),
    created_at   timestamptz not null default now(),
    expires_at   timestamptz not null,
    revoked_at   timestamptz,
    foreign key (account_id, project_id, principal_id)
        references gateway_service_principals (account_id, project_id, id),
    check (expires_at > created_at)
);

CREATE INDEX gateway_project_keys_principal_idx ON gateway_project_keys (account_id, project_id, principal_id);
