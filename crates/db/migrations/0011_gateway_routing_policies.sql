CREATE TABLE gateway_routing_policies (
    account_id text not null references accounts(id),
    project_id text,
    revision int not null check (revision > 0),
    policy jsonb not null check (jsonb_typeof(policy) = 'object'),
    created_at timestamptz not null default now(),
    created_by text not null,
    unique nulls not distinct (account_id, project_id, revision),
    foreign key (account_id, project_id) references gateway_projects (account_id, id)
);
