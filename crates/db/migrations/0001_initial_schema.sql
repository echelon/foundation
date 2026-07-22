-- ============================================================================
-- Foundation — initial schema (all tables declared in one go)
--
-- Faithful to design/ (schema.md §7 is the authoritative table index). Conventions
-- (conventions_database.md): NO triggers, NO REFERENCES/foreign keys — integrity
-- lives in code. Every table opens with `sid bigint generated always as identity`
-- (ORDER BY only, never exposed) + `id text primary key` = {prefix}_{ULID}.
-- Enum-ish columns are text + CHECK; CHECK value sets mirror crates/core/enums.
-- Money is bigint micro-USD (_micros) / nano-USD (_nanos). All timestamps timestamptz.
-- ============================================================================

-- ─────────────────────────── Identity & access (accounts.md §2) ───────────────────────────

CREATE TABLE accounts (
    sid                     bigint generated always as identity,
    id                      text primary key,                       -- acct_{ulid}
    name                    text,
    business_name           text,                                   -- owner/admin-set; null → show the placeholder
    placeholder_business_name text not null,                        -- random "Adjective Noun", minted at creation

    billing_email           text,
    billing_invoice_email   text,
    billing_phone           text,
    billing_address_line_1  text,
    billing_address_line_2  text,
    billing_country_code    text,
    billing_region          text,
    billing_city            text,
    billing_postal_code     text,
    api_version             date,
    status                  text not null CHECK (status IN ('active', 'suspended', 'closed')),
    tos_version             text,
    tos_accepted_at         timestamptz,
    tos_accepted_by_user_id text,
    privacy_version         text,
    privacy_accepted_at     timestamptz,
    created_at              timestamptz not null default now(),
    updated_at              timestamptz not null default now(),
    deleted_at              timestamptz
);

CREATE TABLE users (
    sid                 bigint generated always as identity,
    id                  text primary key,                           -- user_{ulid}
    email               text not null UNIQUE,
    email_clean         text not null,
    email_original      text not null,
    email_verified_at   timestamptz,
    password_hash       text,
    password_version    int not null default 0,
    session_version     int not null default 0,
    display_name        text,
    totp_secret         text,
    is_staff            boolean not null default false,
    tos_version         text,
    tos_accepted_at     timestamptz,
    privacy_version     text,
    privacy_accepted_at timestamptz,
    created_at          timestamptz not null default now(),
    updated_at          timestamptz not null default now(),
    deleted_at          timestamptz
);

CREATE TABLE user_sso_connections (
    sid                        bigint generated always as identity,
    id                         text primary key,                    -- sso_{ulid}
    user_id                    text not null,
    provider                   text not null CHECK (provider IN ('google', 'github')),
    provider_user_id           text not null,
    provider_email             text not null,
    provider_user_display_name text,
    provider_user_first_name   text,
    provider_user_last_name    text,
    provider_username          text,
    created_at                 timestamptz not null default now(),
    last_used_at               timestamptz not null,
    UNIQUE (provider, provider_user_id),
    UNIQUE (user_id, provider)
);

CREATE TABLE account_members (
    sid        bigint generated always as identity,
    id         text primary key,                                    -- mem_{ulid}
    account_id text not null,
    user_id    text not null,
    role       text not null CHECK (role IN ('owner', 'admin', 'member')),
    created_at timestamptz not null default now(),
    UNIQUE (account_id, user_id)
);
CREATE UNIQUE INDEX account_members_one_owner ON account_members (account_id) WHERE role = 'owner';

CREATE TABLE invitations (
    sid                 bigint generated always as identity,
    id                  text primary key,                           -- inv_{ulid}
    account_id          text not null,
    invited_by_user_id  text not null,
    accepted_by_user_id text,
    email               text not null,
    role                text not null CHECK (role IN ('admin', 'member')),
    code                text not null UNIQUE,
    token_hash          text not null,
    created_at          timestamptz not null default now(),
    expires_at          timestamptz not null,
    accepted_at         timestamptz,
    revoked_at          timestamptz
);

CREATE TABLE sessions (
    sid                  bigint generated always as identity,
    id                   text primary key,                          -- sess_{ulid}
    user_id              text not null,
    impersonator_user_id text,
    ip                   inet,
    user_agent           text,
    device               text,
    created_via          text not null CHECK (created_via IN
                             ('signup', 'invited_signup', 'login', 'sso_login', 'sso_signup', 'impersonation')),
    created_at           timestamptz not null default now(),
    expires_at           timestamptz not null,
    revoked_at           timestamptz
);

CREATE TABLE api_keys (
    sid                bigint generated always as identity,
    id                 text primary key,                            -- key_{ulid}
    account_id         text not null,
    created_by_user_id text not null,
    title              text not null,
    description        text,
    key_prefix         text not null,
    key_hash           text not null,
    created_at         timestamptz not null default now(),
    last_used_at       timestamptz,
    expires_at         timestamptz,
    revoked_at         timestamptz
);

CREATE TABLE email_verifications (
    sid          bigint generated always as identity,
    id           text primary key,                                  -- emailver_{ulid}
    user_id      text not null,
    email        text not null,
    token_hash   text not null,
    created_at   timestamptz not null default now(),
    expires_at   timestamptz not null,
    validated_at timestamptz
);

CREATE TABLE password_resets (
    sid        bigint generated always as identity,
    id         text primary key,                                    -- pwreset_{ulid}
    user_id    text not null,
    token_hash text not null,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null,
    used_at    timestamptz
);

CREATE TABLE email_changes (
    sid                bigint generated always as identity,
    id                 text primary key,                            -- emailchg_{ulid}
    user_id            text not null,
    old_email          text not null,
    old_email_clean    text not null,
    old_email_original text not null,
    new_email          text not null,
    new_email_clean    text not null,
    new_email_original text not null,
    change_source      text not null CHECK (change_source IN ('user', 'system')),
    created_at         timestamptz not null default now()
);

CREATE TABLE user_avatars (
    sid                      bigint generated always as identity,
    id                       text primary key,                      -- avatar_{ulid}
    user_id                  text not null UNIQUE,
    custom_file_id           text,
    gravatar_file_id         text,
    -- The USER'S CHOICE of what to display (REVISED 2026-07-20: replaces the derived `source`
    -- + `is_gravatar_dismissed` pair). `not_yet_set` = the signup default, meaning the system
    -- auto-picks: gravatar when one is mirrored, else the letter monogram. An explicit user
    -- choice replaces it and is honored from then on; the effective display is the choice when
    -- it's satisfiable (custom needs custom_file_id, gravatar needs gravatar_file_id), else
    -- the letter monogram.
    preferred_source         text not null default 'not_yet_set' CHECK (preferred_source IN ('not_yet_set', 'custom', 'gravatar', 'letter', 'generated')),
    -- The monogram's gradient, user-picked in settings (2026-07-21); null = default 'brand'.
    monogram_gradient        text CHECK (monogram_gradient IN
                                 ('brand', 'ocean', 'sunset', 'forest', 'berry', 'gold', 'slate', 'rose')),
    gravatar_hash            text,
    gravatar_checked_at      timestamptz,
    generated_seed           text not null,
    email_first_letter       text,
    email_last_letter        text,
    person_name_first_letter text,
    person_name_last_letter  text,
    created_at               timestamptz not null default now(),
    updated_at               timestamptz not null default now()
);

CREATE TABLE user_avatar_files (
    sid                          bigint generated always as identity,
    id                           text primary key,                  -- uaf_{ulid}
    user_id                      text not null,
    original_user_avatar_file_id text,
    avatar_source                text not null CHECK (avatar_source IN ('custom_upload', 'gravatar_mirror')),
    bucket_hash                  text not null,
    content_hash                 text not null,
    bucket_suffix                text,
    bucket_extension             text not null,
    width                        int,
    height                       int,
    bytes                        bigint not null,
    created_at                   timestamptz not null default now()
);

CREATE TABLE user_sso_disconnections (
    sid                        bigint generated always as identity,
    id                         text primary key,                    -- ssodisc_{ulid}
    user_id                    text not null,
    provider                   text not null CHECK (provider IN ('google', 'github')),
    provider_user_id           text not null,
    provider_email             text not null,
    provider_user_display_name text,
    provider_user_first_name   text,
    provider_user_last_name    text,
    provider_username          text,
    connected_at               timestamptz not null,
    disconnected_at            timestamptz not null
);

CREATE TABLE legal_acceptances (
    sid        bigint generated always as identity,
    id         text primary key,                                    -- legal_{ulid}
    user_id    text not null,
    account_id text,
    document   text not null CHECK (document IN ('tos', 'privacy')),
    version    text not null,
    ip         inet,
    created_at timestamptz not null default now()
);
CREATE INDEX legal_acceptances_user_doc_idx ON legal_acceptances (user_id, document, created_at desc);
CREATE INDEX legal_acceptances_account_doc_idx ON legal_acceptances (account_id, document, created_at desc);

CREATE TABLE impersonation_audit_logs (
    sid                  bigint generated always as identity,
    id                   text primary key,                          -- impaudit_{ulid}
    session_id           text,
    impersonator_user_id text not null,
    target_user_id       text not null,
    account_id           text,
    action               text not null CHECK (action IN ('started', 'ended')),
    reason               text,
    ip                   inet,
    created_at           timestamptz not null default now()
);

-- end_users (schema.md §1 / end_users.md §2)
CREATE TABLE end_users (
    sid           bigint generated always as identity,
    id            text primary key,                                 -- enduser_{ulid}
    account_id    text not null,
    external_id   text not null,
    first_seen_at timestamptz not null,
    last_seen_at  timestamptz not null,
    metadata      jsonb,
    UNIQUE (account_id, external_id)
);

-- ─────────────────────────── Jobs (jobs.md §4) ───────────────────────────

CREATE TABLE jobs (
    sid                  bigint generated always as identity,
    id                   text primary key,                          -- job_{ulid}
    account_id           text not null,
    api_key_id           text,                                      -- null = dashboard session
    user_id              text,
    end_user_id          text,
    source               text not null CHECK (source IN ('api_key', 'dashboard', 'mcp')),
    media_type           text not null CHECK (media_type IN ('image', 'video', 'audio', 'mesh', 'splat', 'text')),
    model                text not null,
    modality             text not null,                             -- enum: Modality (24, models.md §11)
    api_shape            text not null CHECK (api_shape IN ('granular', 'unified', 'any', 'chat')),
    api_version          date not null,
    request_params       jsonb not null,
    param_adjustments    jsonb not null,
    is_dry_run           bool not null default false,               -- dry_run request: validated/priced/logged, never sent to the provider (born terminal)
    retry_policy         jsonb,
    status               text not null CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'canceled', 'expired')),
    queue_stage          text CHECK (queue_stage IN ('at_our_gateway', 'at_provider')),
    dispatch_attempts    int not null,
    failure_code         text,                                      -- enum: FailureCode (api_overview.md §2)
    failure_message      text,
    canceled_by          text CHECK (canceled_by IN ('user', 'system', 'provider')),
    provider             text,                                      -- enum: Provider; null until dispatch
    provider_account     text,
    provider_job_id      text,
    billing_mode         text not null CHECK (billing_mode IN ('fixed', 'metered')),
    billing_state        text not null CHECK (billing_state IN ('pending_charge', 'charged', 'refunded_partial', 'refunded_full', 'not_charged')),
    price_unit           text not null CHECK (price_unit IN ('per_call', 'per_image', 'per_video_second', 'per_audio_second', 'per_megapixel', 'per_second_compute', 'per_million_tokens_in', 'per_million_tokens_out', 'per_mesh', 'per_splat')),
    unit_amount_nanos    bigint not null,
    quantity             numeric,
    charged_micros       bigint,
    estimated_max_micros bigint,
    refunded_micros      bigint not null,
    webhook_url          text,
    webhook_events       text[],
    created_at           timestamptz not null default now(),
    next_dispatch_at     timestamptz,
    dispatched_at        timestamptz,
    started_at           timestamptz,
    completed_at         timestamptz,
    expires_at           timestamptz not null
);
CREATE INDEX jobs_account_created_idx ON jobs (account_id, created_at desc);
CREATE INDEX jobs_account_status_idx  ON jobs (account_id, status);
CREATE INDEX jobs_provider_jobid_idx  ON jobs (provider, provider_job_id);
CREATE INDEX jobs_status_expires_idx  ON jobs (status, expires_at);
CREATE INDEX jobs_account_enduser_idx ON jobs (account_id, end_user_id, created_at) WHERE end_user_id IS NOT NULL;

CREATE TABLE job_events (
    sid        bigint generated always as identity,
    id         text primary key,                                    -- jevt_{ulid}
    job_id     text not null,
    event_type text not null CHECK (event_type IN ('created', 'charged', 'dispatch_attempted', 'provider_accepted', 'poll_tick', 'provider_completed', 'output_ingested', 'webhook_sent', 'refunded', 'failed', 'canceled', 'expired')),
    data       jsonb,
    created_at timestamptz not null default now()
);

-- job_slices (NEW 2026-07-22, jobs.md addendum): fan-out units for jobs whose ONE API
-- request maps to MULTIPLE provider calls (e.g. GPT Image n>1 → n background responses,
-- since OpenAI's Responses image tool has no `n`). Rows exist ONLY for fanned-out jobs —
-- single-call jobs keep provider state on the jobs row itself. `slice_index` maps 1:1 to
-- `outputs.output_index`, which is how partial completion surfaces: outputs appear as each
-- slice lands. The job goes `succeeded` when ≥1 slice succeeds (partial results beat none);
-- `failed` only when every slice failed.
CREATE TABLE job_slices (
    sid             bigint generated always as identity,
    id              text primary key,                              -- slice_{ulid}
    job_id          text not null,
    slice_index     int not null,
    status          text not null CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'canceled')),
    provider_job_id text,                                          -- e.g. OpenAI resp_…
    failure_code    text,
    failure_message text,
    created_at      timestamptz not null default now(),
    dispatched_at   timestamptz,
    completed_at    timestamptz,
    UNIQUE (job_id, slice_index)
);
CREATE INDEX job_slices_job_idx             ON job_slices (job_id);
CREATE INDEX job_slices_provider_jobid_idx  ON job_slices (provider_job_id);

-- ─────────────────────────── Files & outputs (files.md §2) ───────────────────────────

CREATE TABLE files (
    sid              bigint generated always as identity,
    id               text primary key,                             -- file_{ulid}
    account_id       text not null,
    purpose          text not null CHECK (purpose IN ('input', 'output')),
    origin           text not null CHECK (origin IN ('upload', 'mirrored_url', 'generated')),
    origin_url       text,
    bucket_folder    text not null CHECK (bucket_folder IN ('generations', 'uploads', 'samples', 'avatars', 'invoices')),
    bucket_hash      text not null,
    bucket_prefix    text,
    bucket_suffix    text,
    bucket_version   int,
    bucket_extension text,
    mime             text not null,
    bytes            bigint not null,
    checksum_sha256  text not null,
    storage_class    text not null CHECK (storage_class IN ('hot', 'cold')),
    created_at       timestamptz not null default now()
);

CREATE TABLE outputs (
    sid          bigint generated always as identity,
    id           text primary key,                                 -- out_{ulid}
    job_id       text not null,
    file_id      text,                                             -- null only for inline-text outputs
    output_index int not null,
    kind         text not null CHECK (kind IN ('image', 'video', 'audio', 'mesh', 'splat', 'text')),
    width        int,
    height       int,
    duration_ms  int,
    text_content text,
    created_at   timestamptz not null default now(),
    CHECK (file_id IS NOT NULL OR text_content IS NOT NULL),
    UNIQUE (job_id, output_index)
);

-- asset_ingests (assets.md §2.1)
CREATE TABLE asset_ingests (
    sid           bigint generated always as identity,
    id            text primary key,                                -- ingest_{ulid}
    account_id    text not null,
    file_id       text,
    source        text not null CHECK (source IN ('upload', 'url')),
    source_url    text,
    status        text not null CHECK (status IN ('pending', 'fetching', 'stored', 'rejected', 'failed')),
    reject_reason text,
    media_type    text not null CHECK (media_type IN ('image', 'video', 'audio', 'mesh', 'splat', 'text')),
    mime          text,
    bytes         bigint,
    created_at    timestamptz not null default now(),
    completed_at  timestamptz
);
CREATE INDEX asset_ingests_account_created_idx ON asset_ingests (account_id, created_at desc);
CREATE INDEX asset_ingests_source_url_idx      ON asset_ingests (source_url);

-- ─────────────────────────── Queue (schema.md §3) ───────────────────────────

CREATE TABLE queue_tasks (
    sid          bigint generated always as identity,
    id           text primary key,                                 -- task_{ulid}
    task_type    text not null CHECK (task_type IN ('dispatch_job', 'poll_provider', 'ingest_outputs', 'deliver_webhook', 'settle_refund', 'auto_refill', 'rollup_usage', 'process_stripe_event', 'fetch_asset', 'send_email', 'generate_invoice')),
    payload      jsonb not null,
    priority     int not null,
    attempts     int not null,
    max_attempts int not null,
    locked_by    text,
    status       text not null CHECK (status IN ('pending', 'running', 'done', 'dead')),
    last_error   text,
    created_at   timestamptz not null default now(),
    run_at       timestamptz not null,
    locked_at    timestamptz
) WITH (fillfactor = 80);
CREATE INDEX queue_tasks_worker_loop_idx ON queue_tasks (status, run_at, priority);

-- ─────────────────────────── Money: payments (payments.md §2) ───────────────────────────

CREATE TABLE stripe_customers (
    sid                bigint generated always as identity,
    id                 text primary key,                           -- cust_{ulid}
    account_id         text not null UNIQUE,
    stripe_customer_id text not null UNIQUE,
    created_at         timestamptz not null default now()
);

CREATE TABLE payment_methods (
    sid                      bigint generated always as identity,
    id                       text primary key,                     -- pm_{ulid}
    account_id               text not null,
    stripe_payment_method_id text not null,
    brand                    text,
    last4                    text,
    exp_month                int,
    exp_year                 int,
    is_default               bool not null,
    status                   text not null CHECK (status IN ('active', 'detached')),
    created_at               timestamptz not null default now()
);
CREATE UNIQUE INDEX payment_methods_one_default ON payment_methods (account_id) WHERE is_default AND status = 'active';

-- ledger_entries — append-only
CREATE TABLE ledger_entries (
    sid                      bigint generated always as identity,
    id                       text primary key,                     -- txn_{ulid}
    account_id               text not null,
    job_id                   text,
    amount_micros            bigint not null,                      -- signed: + credit, - debit
    entry_type               text not null CHECK (entry_type IN ('topup', 'job_charge', 'job_refund', 'adjustment', 'promo', 'auto_refill', 'chargeback', 'chargeback_reversal')),
    stripe_payment_intent_id text,
    balance_after_micros     bigint not null,
    memo                     text,
    idempotency_key          text UNIQUE,
    created_at               timestamptz not null default now()
);

CREATE TABLE account_balances (
    sid            bigint generated always as identity,
    account_id     text primary key,                               -- one row per account
    balance_micros bigint not null,
    updated_at     timestamptz not null
);

CREATE TABLE auto_refill_rules (
    sid                  bigint generated always as identity,
    id                   text primary key,                         -- refill_{ulid}
    account_id           text not null UNIQUE,
    payment_method_id    text,
    is_enabled           bool not null,
    threshold_micros     bigint not null,
    refill_amount_micros bigint not null,
    daily_max_micros     bigint not null,
    consecutive_failures int not null,
    created_at           timestamptz not null default now(),
    last_triggered_at    timestamptz,
    updated_at           timestamptz not null
);

CREATE TABLE spend_limits (
    sid                bigint generated always as identity,
    id                 text primary key,                           -- limit_{ulid}
    account_id         text not null,
    created_by_user_id text not null,
    "window"           text not null CHECK ("window" IN ('weekly', 'monthly')),
    limit_micros       bigint not null,
    behavior           text not null CHECK (behavior IN ('block')),
    period_start       timestamptz not null,
    spent_micros       bigint not null,
    created_at         timestamptz not null default now(),
    updated_at         timestamptz not null
);

CREATE TABLE stripe_events (
    sid             bigint generated always as identity,
    id              text primary key,                              -- sevt_{ulid}
    stripe_event_id text not null UNIQUE,
    event_type      text not null,
    payload         jsonb not null,
    created_at      timestamptz not null default now(),
    processed_at    timestamptz,
    error           text
);

-- ─────────────────────────── Money: billing (billing.md §8, §12) ───────────────────────────

-- billing_events — append-only
CREATE TABLE billing_events (
    sid                      bigint generated always as identity,
    id                       text primary key,                     -- bevt_{ulid}
    account_id               text not null,
    actor_user_id            text,
    impersonation_session_id text,
    job_id                   text,
    ledger_entry_id          text,
    payment_method_id        text,
    event                    text not null,                        -- enum: BillingEvent (20 values)
    actor_type               text not null CHECK (actor_type IN ('user', 'system', 'stripe', 'support')),
    data                     jsonb,
    created_at               timestamptz not null default now()
);
CREATE INDEX billing_events_account_created ON billing_events (account_id, created_at desc);
CREATE INDEX billing_events_job ON billing_events (job_id);

CREATE TABLE invoices (
    sid            bigint generated always as identity,
    id             text primary key,                               -- invoice_{ulid}
    account_id     text not null,
    pdf_file_id    text,
    period_start   timestamptz not null,
    period_end     timestamptz not null,
    number         text not null UNIQUE,                           -- INV-000042, gapless
    currency       text not null,
    opening_micros bigint not null,
    topups_micros  bigint not null,
    spend_micros   bigint not null,
    closing_micros bigint not null,
    status         text not null CHECK (status IN ('generating', 'issued', 'mailed', 'void')),
    issued_at      timestamptz,
    created_at     timestamptz not null default now(),
    UNIQUE (account_id, period_start)
);
CREATE INDEX invoices_account_period ON invoices (account_id, period_start desc);

CREATE TABLE invoice_counters (
    sid         bigint generated always as identity,
    scope       text primary key,                                  -- 'global'
    next_number bigint not null,
    updated_at  timestamptz not null
);

-- ─────────────────────────── Logs (logging.md §3) — partitioned, high-volume ───────────────────────────

CREATE TABLE request_logs (
    sid                   bigint generated always as identity,
    id                    text not null,                           -- rlog_{ulid}
    account_id            text,
    api_key_id            text,
    session_id            text,
    job_id                text,
    end_user_id           text,
    source                text not null CHECK (source IN ('api_key', 'dashboard', 'mcp')),
    method                text not null,
    path                  text not null,
    api_version           date,
    ip                    inet,
    user_agent            text,
    request_headers       jsonb,
    request_body          jsonb,
    is_request_truncated  bool not null,
    request_body_bytes    bigint,
    response_status       int,
    response_body         jsonb,
    is_response_truncated bool not null,
    response_body_bytes   bigint,
    latency_ms            int,
    created_at            timestamptz not null default now(),
    primary key (id, created_at)
) PARTITION BY RANGE (created_at);
CREATE TABLE request_logs_default PARTITION OF request_logs DEFAULT;
CREATE INDEX request_logs_account_created_idx ON request_logs (account_id, created_at);
CREATE INDEX request_logs_job_idx ON request_logs (job_id);

-- provider_calls: the outbound third-party request log (logging.md §3). `id` IS the
-- request_id — minted BEFORE the send (so failures still capture the payload) and used as
-- the correlation key for debug aggregation. Addendum 2026-07-22: job_slice_id (fan-out),
-- provider_job_id (the provider's handle, e.g. OpenAI resp_… — how inbound webhooks find
-- their originating request), is_dry_run (payload serialized + logged, never sent).
CREATE TABLE provider_calls (
    sid                   bigint generated always as identity,
    id                    text not null,                           -- pcall_{ulid}
    job_id                text not null,
    job_slice_id          text,                                    -- set when the call belongs to one fan-out slice
    provider              text not null,
    direction             text not null CHECK (direction IN ('request', 'response', 'poll_request', 'poll_response', 'callback_in')),
    is_dry_run            bool not null default false,
    provider_job_id       text,                                    -- provider handle from the response (e.g. resp_…)
    url                   text,
    request_headers       jsonb,
    request_body          jsonb,
    is_request_truncated  bool not null,
    request_body_bytes    bigint,
    response_status       int,
    response_body         jsonb,
    is_response_truncated bool not null,
    response_body_bytes   bigint,
    latency_ms            int,
    created_at            timestamptz not null default now(),
    primary key (id, created_at)
) PARTITION BY RANGE (created_at);
CREATE TABLE provider_calls_default PARTITION OF provider_calls DEFAULT;
CREATE INDEX provider_calls_job_idx ON provider_calls (job_id);
CREATE INDEX provider_calls_provider_jobid_idx ON provider_calls (provider_job_id) WHERE provider_job_id IS NOT NULL;

CREATE TABLE webhook_deliveries (
    sid             bigint generated always as identity,
    id              text not null,                                 -- whd_{ulid}
    job_id          text not null,
    account_id      text not null,
    url             text not null,
    event_type      text not null CHECK (event_type IN ('job_completed', 'job_failed')),
    payload         jsonb not null,
    attempt         int not null,
    response_status int,
    response_body   text,
    is_truncated    bool not null,
    latency_ms      int,
    created_at      timestamptz not null default now(),
    next_retry_at   timestamptz,
    delivered_at    timestamptz,
    gave_up_at      timestamptz,
    primary key (id, created_at)
) PARTITION BY RANGE (created_at);
CREATE TABLE webhook_deliveries_default PARTITION OF webhook_deliveries DEFAULT;
CREATE INDEX webhook_deliveries_account_created_idx ON webhook_deliveries (account_id, created_at);
CREATE INDEX webhook_deliveries_job_idx ON webhook_deliveries (job_id);

-- inbound_webhooks (NEW 2026-07-22, logging.md addendum): one row per provider→us webhook
-- delivery — EVERY delivery, verified or not (rejected signatures are security signal).
-- Correlation back to the originating outbound request goes through
-- provider_calls.provider_job_id (`object_id` here is the same provider handle, e.g.
-- OpenAI resp_…); provider_call_id is that request's id (the request_id).
CREATE TABLE inbound_webhooks (
    sid                bigint generated always as identity,
    id                 text not null,                              -- iwh_{ulid}
    provider           text not null,
    event_id           text,                                       -- provider's event id (evt_…); dedup key
    event_type         text,                                       -- e.g. response.completed
    object_id          text,                                       -- subject id (e.g. resp_…)
    is_signature_valid bool not null,
    provider_call_id   text,                                       -- correlated outbound request (request_id), when found
    job_id             text,
    job_slice_id       text,
    outcome            text not null CHECK (outcome IN ('processed', 'unmatched', 'duplicate', 'ignored', 'signature_failed', 'not_configured', 'error')),
    payload            jsonb,
    created_at         timestamptz not null default now(),
    primary key (id, created_at)
) PARTITION BY RANGE (created_at);
CREATE TABLE inbound_webhooks_default PARTITION OF inbound_webhooks DEFAULT;
CREATE INDEX inbound_webhooks_object_idx ON inbound_webhooks (object_id);
CREATE INDEX inbound_webhooks_event_idx  ON inbound_webhooks (provider, event_id);

-- email_sends (email.md §4)
CREATE TABLE email_sends (
    sid                 bigint generated always as identity,
    id                  text primary key,                          -- mail_{ulid}
    user_id             text,
    to_email            text not null,
    template            text not null CHECK (template IN ('invite', 'password_reset', 'email_verify', 'autorefill_failed', 'account_suspended')),
    status              text not null CHECK (status IN ('queued', 'sent', 'delivered', 'bounced', 'complained', 'failed')),
    provider_message_id text,
    error               text,
    created_at          timestamptz not null default now(),
    updated_at          timestamptz not null default now()
);

-- ─────────────────────────── Security (schema.md §5) ───────────────────────────

CREATE TABLE webhook_secrets (
    sid        bigint generated always as identity,
    id         text primary key,                                   -- whs_{ulid}
    account_id text not null,
    secret     text not null,                                      -- foundation_whsec_..., AES-GCM at rest
    created_at timestamptz not null default now(),
    revoked_at timestamptz
);

CREATE TABLE idempotency_keys (
    sid               bigint generated always as identity,
    id                text primary key,                            -- idem_{ulid}
    account_id        text not null,
    key               text not null,
    request_hash      text not null,
    response_snapshot jsonb not null,
    created_at        timestamptz not null default now(),
    expires_at        timestamptz not null,
    UNIQUE (account_id, key)
);

-- ─────────────────────────── Audit (audit_logs.md §2) ───────────────────────────

CREATE TABLE user_audit_logs (
    sid                      bigint generated always as identity,
    id                       text primary key,                     -- uaudit_{ulid}
    user_id                  text not null,
    actor_user_id            text,
    impersonation_session_id text,
    event                    text not null,                        -- enum: AuditEvent (21)
    target_type              text,                                 -- enum: AuditTargetType
    target_id                text,
    data                     jsonb,
    is_org_visible           bool not null,
    ip                       inet,
    created_at               timestamptz not null default now()
);
CREATE INDEX user_audit_logs_user_created_idx ON user_audit_logs (user_id, created_at desc);
CREATE INDEX user_audit_logs_user_created_orgvis_idx ON user_audit_logs (user_id, created_at desc) WHERE is_org_visible;

CREATE TABLE account_audit_logs (
    sid                      bigint generated always as identity,
    id                       text primary key,                     -- aaudit_{ulid}
    account_id               text not null,
    actor_user_id            text,
    impersonation_session_id text,
    user_id                  text,
    event                    text not null,                        -- enum: AuditEvent (21)
    target_type              text,                                 -- enum: AuditTargetType
    target_id                text,
    data                     jsonb,
    ip                       inet,
    created_at               timestamptz not null default now()
);
CREATE INDEX account_audit_logs_account_created_idx ON account_audit_logs (account_id, created_at desc);

-- ─────────────────────────── Rollups (model_rankings_and_reports.md §5) ───────────────────────────

CREATE TABLE usage_rollups (
    sid               bigint generated always as identity,
    id                text primary key,                            -- roll_{ulid}
    grain             text not null CHECK (grain IN ('hour', 'day', 'week', 'month')),
    period_start      timestamptz not null,
    media_type        text not null CHECK (media_type IN ('image', 'video', 'audio', 'mesh', 'splat', 'text')),
    model             text not null,
    model_owner       text not null,
    modality          text not null,
    provider          text not null,
    source            text not null,
    jobs_total        bigint not null,
    jobs_succeeded    bigint not null,
    jobs_failed       bigint not null,
    output_units      numeric not null,
    tokens_in         bigint,
    tokens_out        bigint,
    distinct_accounts int not null,                                -- PRIVATE — K-anon gate
    spend_micros      bigint not null,                             -- PRIVATE — internal only
    p50_latency_ms    int,
    p95_latency_ms    int,
    created_at        timestamptz not null default now(),
    UNIQUE (grain, period_start, media_type, model, modality, provider, source)
);

CREATE TABLE usage_rollups_category (
    sid               bigint generated always as identity,
    id                text primary key,                            -- rollc_{ulid}
    grain             text not null CHECK (grain IN ('hour', 'day', 'week', 'month')),
    period_start      timestamptz not null,
    media_type        text not null,
    model             text not null,
    model_owner       text not null,
    content_category  text not null,
    jobs_classified   bigint not null,
    jobs_succeeded    bigint not null,
    output_units      numeric not null,
    distinct_accounts int not null,                                -- PRIVATE
    created_at        timestamptz not null default now(),
    UNIQUE (grain, period_start, media_type, model, content_category)
);

CREATE TABLE usage_rollups_app (
    sid          bigint generated always as identity,
    id           text primary key,                                 -- rolla_{ulid}
    grain        text not null CHECK (grain IN ('hour', 'day', 'week', 'month')),
    period_start timestamptz not null,
    media_type   text not null,
    model        text not null,
    app_id       text not null,                                    -- opt-in public app label
    jobs_total   bigint not null,
    output_units numeric not null,
    created_at   timestamptz not null default now(),
    UNIQUE (grain, period_start, media_type, model, app_id)
);

CREATE TABLE usage_rollups_language (
    sid               bigint generated always as identity,
    id                text primary key,                            -- rolll_{ulid}
    grain             text not null CHECK (grain IN ('hour', 'day', 'week', 'month')),
    period_start      timestamptz not null,
    media_type        text not null CHECK (media_type IN ('text', 'audio')),
    model             text not null,
    model_owner       text not null,
    language_kind     text not null CHECK (language_kind IN ('programming', 'natural')),
    language          text not null,
    jobs_classified   bigint not null,
    output_units      numeric not null,
    distinct_accounts int not null,                                -- PRIVATE
    created_at        timestamptz not null default now(),
    UNIQUE (grain, period_start, media_type, model, language_kind, language)
);

-- model_usage_summary — the HOT catalog-index digest (~500 rows), all PUBLIC
CREATE TABLE model_usage_summary (
    sid               bigint generated always as identity,
    id                text primary key,                            -- sum_{ulid}
    model             text not null UNIQUE,
    media_type        text not null,
    model_owner       text not null,
    window_days       int not null,
    generations       bigint not null,
    output_units      numeric not null,
    list_value_micros bigint not null,                             -- list price × volume, PUBLIC (not revenue)
    wow_generations   numeric not null,
    rank_generations  int not null,
    rank_output       int not null,
    rank_value        int not null,
    sparkline         int[] not null,
    success_rate      numeric,
    updated_at        timestamptz not null
);

-- content_labels — sparse per-job classification (category + languages)
CREATE TABLE content_labels (
    sid                  bigint generated always as identity,
    id                   text primary key,                         -- cl_{ulid}
    job_id               text not null,
    media_type           text not null,
    content_category     text,                                     -- null when only language classified
    programming_language text,
    natural_language     text,
    confidence           numeric not null,
    classifier_model     text not null,
    is_opted_out         bool not null,
    classified_at        timestamptz not null,
    created_at           timestamptz not null default now(),
    UNIQUE (job_id)
);
