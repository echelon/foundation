ALTER TABLE gateway_export_snapshots ADD UNIQUE (account_id, project_id, id);

CREATE TABLE gateway_archives (
    id                 text primary key,
    account_id         text not null,
    project_id         text not null,
    client_ref         text,
    export_id          text not null,
    requested_by       text not null,
    after_seq          bigint not null check (after_seq >= 0),
    through_seq        bigint not null check (through_seq >= after_seq),
    state              text not null default 'queued' check (state IN (
        'queued', 'sealing', 'stored', 'restore_queued', 'restoring', 'available', 'archive_failed', 'restore_failed'
    )),
    generation         integer not null default 0 check (generation >= 0),
    tries              integer not null default 0 check (tries between 0 and 3),
    lease_until        timestamptz,
    storage_generation integer,
    schema_version     integer not null default 1 check (schema_version > 0),
    event_count        integer check (event_count between 1 and 1000),
    first_event_id     text,
    last_event_id      text,
    has_more          boolean,
    content_sha256    text check (length(content_sha256) = 64),
    object_sha256     text check (length(object_sha256) = 64),
    error_code        text,
    created_at        timestamptz not null default clock_timestamp(),
    stored_at         timestamptz,
    restored_until    timestamptz,
    foreign key (account_id, project_id) references gateway_projects (account_id, id),
    foreign key (account_id, project_id, export_id) references gateway_export_snapshots (account_id, project_id, id),
    unique (export_id, after_seq),
    check ((state IN ('sealing', 'restoring')) = (lease_until IS NOT NULL)),
    check ((state IN ('stored','restore_queued','restoring','available','restore_failed')) = (stored_at IS NOT NULL)),
    check ((state = 'available') = (restored_until IS NOT NULL)),
    check ((stored_at IS NOT NULL) = (
        storage_generation IS NOT NULL AND event_count IS NOT NULL AND first_event_id IS NOT NULL
        AND last_event_id IS NOT NULL AND has_more IS NOT NULL AND content_sha256 IS NOT NULL AND object_sha256 IS NOT NULL
    ))
);
CREATE INDEX gateway_archives_scope_idx ON gateway_archives (account_id, project_id, id DESC);
CREATE INDEX gateway_archives_ready_idx ON gateway_archives (created_at, id)
    WHERE state IN ('queued','sealing','restore_queued','restoring');
