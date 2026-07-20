-- ============================================================================
-- MATERIALIZED SCHEMA ROLLUP — generated, do not edit by hand.
-- Regenerate: script/dump-schema.sh
-- ============================================================================

--
-- PostgreSQL database dump
--

\restrict 6QFBWPdccQH2gTTBPNMNcbgRmZLI7QjKnz1t1PklvF6laO4SjONywTtog9A8SGm

-- Dumped from database version 16.14 (Homebrew)
-- Dumped by pg_dump version 16.14 (Homebrew)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: account_audit_logs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.account_audit_logs (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    actor_user_id text,
    impersonation_session_id text,
    user_id text,
    event text NOT NULL,
    target_type text,
    target_id text,
    data jsonb,
    ip inet,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: account_audit_logs_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.account_audit_logs ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.account_audit_logs_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: account_balances; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.account_balances (
    sid bigint NOT NULL,
    account_id text NOT NULL,
    balance_micros bigint NOT NULL,
    updated_at timestamp with time zone NOT NULL
);


--
-- Name: account_balances_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.account_balances ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.account_balances_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: account_members; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.account_members (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    user_id text NOT NULL,
    role text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT account_members_role_check CHECK ((role = ANY (ARRAY['owner'::text, 'admin'::text, 'member'::text])))
);


--
-- Name: account_members_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.account_members ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.account_members_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: accounts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.accounts (
    sid bigint NOT NULL,
    id text NOT NULL,
    name text,
    business_name text,
    placeholder_business_name text NOT NULL,
    billing_email text,
    billing_invoice_email text,
    billing_phone text,
    billing_address_line_1 text,
    billing_address_line_2 text,
    billing_country_code text,
    billing_region text,
    billing_city text,
    billing_postal_code text,
    api_version date,
    status text NOT NULL,
    tos_version text,
    tos_accepted_at timestamp with time zone,
    tos_accepted_by_user_id text,
    privacy_version text,
    privacy_accepted_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    deleted_at timestamp with time zone,
    CONSTRAINT accounts_status_check CHECK ((status = ANY (ARRAY['active'::text, 'suspended'::text, 'closed'::text])))
);


--
-- Name: accounts_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.accounts ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.accounts_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: api_keys; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.api_keys (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    created_by_user_id text NOT NULL,
    title text NOT NULL,
    description text,
    key_prefix text NOT NULL,
    key_hash text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used_at timestamp with time zone,
    expires_at timestamp with time zone,
    revoked_at timestamp with time zone
);


--
-- Name: api_keys_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.api_keys ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.api_keys_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: asset_ingests; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.asset_ingests (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    file_id text,
    source text NOT NULL,
    source_url text,
    status text NOT NULL,
    reject_reason text,
    media_type text NOT NULL,
    mime text,
    bytes bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone,
    CONSTRAINT asset_ingests_media_type_check CHECK ((media_type = ANY (ARRAY['image'::text, 'video'::text, 'audio'::text, 'mesh'::text, 'splat'::text, 'text'::text]))),
    CONSTRAINT asset_ingests_source_check CHECK ((source = ANY (ARRAY['upload'::text, 'url'::text]))),
    CONSTRAINT asset_ingests_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'fetching'::text, 'stored'::text, 'rejected'::text, 'failed'::text])))
);


--
-- Name: asset_ingests_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.asset_ingests ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.asset_ingests_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: auto_refill_rules; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.auto_refill_rules (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    payment_method_id text,
    is_enabled boolean NOT NULL,
    threshold_micros bigint NOT NULL,
    refill_amount_micros bigint NOT NULL,
    daily_max_micros bigint NOT NULL,
    consecutive_failures integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_triggered_at timestamp with time zone,
    updated_at timestamp with time zone NOT NULL
);


--
-- Name: auto_refill_rules_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.auto_refill_rules ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.auto_refill_rules_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: billing_events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.billing_events (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    actor_user_id text,
    impersonation_session_id text,
    job_id text,
    ledger_entry_id text,
    payment_method_id text,
    event text NOT NULL,
    actor_type text NOT NULL,
    data jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT billing_events_actor_type_check CHECK ((actor_type = ANY (ARRAY['user'::text, 'system'::text, 'stripe'::text, 'support'::text])))
);


--
-- Name: billing_events_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.billing_events ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.billing_events_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: content_labels; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.content_labels (
    sid bigint NOT NULL,
    id text NOT NULL,
    job_id text NOT NULL,
    media_type text NOT NULL,
    content_category text,
    programming_language text,
    natural_language text,
    confidence numeric NOT NULL,
    classifier_model text NOT NULL,
    is_opted_out boolean NOT NULL,
    classified_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: content_labels_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.content_labels ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.content_labels_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: email_changes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.email_changes (
    sid bigint NOT NULL,
    id text NOT NULL,
    user_id text NOT NULL,
    old_email text NOT NULL,
    old_email_clean text NOT NULL,
    old_email_original text NOT NULL,
    new_email text NOT NULL,
    new_email_clean text NOT NULL,
    new_email_original text NOT NULL,
    change_source text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT email_changes_change_source_check CHECK ((change_source = ANY (ARRAY['user'::text, 'system'::text])))
);


--
-- Name: email_changes_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.email_changes ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.email_changes_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: email_sends; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.email_sends (
    sid bigint NOT NULL,
    id text NOT NULL,
    user_id text,
    to_email text NOT NULL,
    template text NOT NULL,
    status text NOT NULL,
    provider_message_id text,
    error text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT email_sends_status_check CHECK ((status = ANY (ARRAY['queued'::text, 'sent'::text, 'delivered'::text, 'bounced'::text, 'complained'::text, 'failed'::text]))),
    CONSTRAINT email_sends_template_check CHECK ((template = ANY (ARRAY['invite'::text, 'password_reset'::text, 'email_verify'::text, 'autorefill_failed'::text, 'account_suspended'::text])))
);


--
-- Name: email_sends_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.email_sends ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.email_sends_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: email_verifications; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.email_verifications (
    sid bigint NOT NULL,
    id text NOT NULL,
    user_id text NOT NULL,
    email text NOT NULL,
    token_hash text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    validated_at timestamp with time zone
);


--
-- Name: email_verifications_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.email_verifications ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.email_verifications_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: end_users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.end_users (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    external_id text NOT NULL,
    first_seen_at timestamp with time zone NOT NULL,
    last_seen_at timestamp with time zone NOT NULL,
    metadata jsonb
);


--
-- Name: end_users_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.end_users ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.end_users_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: files; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.files (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    purpose text NOT NULL,
    origin text NOT NULL,
    origin_url text,
    bucket_folder text NOT NULL,
    bucket_hash text NOT NULL,
    bucket_prefix text,
    bucket_suffix text,
    bucket_version integer,
    bucket_extension text,
    mime text NOT NULL,
    bytes bigint NOT NULL,
    checksum_sha256 text NOT NULL,
    storage_class text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT files_bucket_folder_check CHECK ((bucket_folder = ANY (ARRAY['generations'::text, 'uploads'::text, 'samples'::text, 'avatars'::text, 'invoices'::text]))),
    CONSTRAINT files_origin_check CHECK ((origin = ANY (ARRAY['upload'::text, 'mirrored_url'::text, 'generated'::text]))),
    CONSTRAINT files_purpose_check CHECK ((purpose = ANY (ARRAY['input'::text, 'output'::text]))),
    CONSTRAINT files_storage_class_check CHECK ((storage_class = ANY (ARRAY['hot'::text, 'cold'::text])))
);


--
-- Name: files_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.files ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.files_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: idempotency_keys; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.idempotency_keys (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    key text NOT NULL,
    request_hash text NOT NULL,
    response_snapshot jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone NOT NULL
);


--
-- Name: idempotency_keys_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.idempotency_keys ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.idempotency_keys_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: impersonation_audit_logs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.impersonation_audit_logs (
    sid bigint NOT NULL,
    id text NOT NULL,
    session_id text,
    impersonator_user_id text NOT NULL,
    target_user_id text NOT NULL,
    account_id text,
    action text NOT NULL,
    reason text,
    ip inet,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT impersonation_audit_logs_action_check CHECK ((action = ANY (ARRAY['started'::text, 'ended'::text])))
);


--
-- Name: impersonation_audit_logs_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.impersonation_audit_logs ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.impersonation_audit_logs_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: invitations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.invitations (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    invited_by_user_id text NOT NULL,
    accepted_by_user_id text,
    email text NOT NULL,
    role text NOT NULL,
    code text NOT NULL,
    token_hash text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    accepted_at timestamp with time zone,
    revoked_at timestamp with time zone,
    CONSTRAINT invitations_role_check CHECK ((role = ANY (ARRAY['admin'::text, 'member'::text])))
);


--
-- Name: invitations_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.invitations ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.invitations_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: invoice_counters; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.invoice_counters (
    sid bigint NOT NULL,
    scope text NOT NULL,
    next_number bigint NOT NULL,
    updated_at timestamp with time zone NOT NULL
);


--
-- Name: invoice_counters_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.invoice_counters ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.invoice_counters_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: invoices; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.invoices (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    pdf_file_id text,
    period_start timestamp with time zone NOT NULL,
    period_end timestamp with time zone NOT NULL,
    number text NOT NULL,
    currency text NOT NULL,
    opening_micros bigint NOT NULL,
    topups_micros bigint NOT NULL,
    spend_micros bigint NOT NULL,
    closing_micros bigint NOT NULL,
    status text NOT NULL,
    issued_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT invoices_status_check CHECK ((status = ANY (ARRAY['generating'::text, 'issued'::text, 'mailed'::text, 'void'::text])))
);


--
-- Name: invoices_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.invoices ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.invoices_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: job_events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.job_events (
    sid bigint NOT NULL,
    id text NOT NULL,
    job_id text NOT NULL,
    event_type text NOT NULL,
    data jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT job_events_event_type_check CHECK ((event_type = ANY (ARRAY['created'::text, 'charged'::text, 'dispatch_attempted'::text, 'provider_accepted'::text, 'poll_tick'::text, 'provider_completed'::text, 'output_ingested'::text, 'webhook_sent'::text, 'refunded'::text, 'failed'::text, 'canceled'::text, 'expired'::text])))
);


--
-- Name: job_events_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.job_events ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.job_events_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: jobs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.jobs (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    api_key_id text,
    user_id text,
    end_user_id text,
    source text NOT NULL,
    media_type text NOT NULL,
    model text NOT NULL,
    modality text NOT NULL,
    api_shape text NOT NULL,
    api_version date NOT NULL,
    request_params jsonb NOT NULL,
    param_adjustments jsonb NOT NULL,
    retry_policy jsonb,
    status text NOT NULL,
    queue_stage text,
    dispatch_attempts integer NOT NULL,
    failure_code text,
    failure_message text,
    canceled_by text,
    provider text,
    provider_account text,
    provider_job_id text,
    billing_mode text NOT NULL,
    billing_state text NOT NULL,
    price_unit text NOT NULL,
    unit_amount_nanos bigint NOT NULL,
    quantity numeric,
    charged_micros bigint,
    estimated_max_micros bigint,
    refunded_micros bigint NOT NULL,
    webhook_url text,
    webhook_events text[],
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    next_dispatch_at timestamp with time zone,
    dispatched_at timestamp with time zone,
    started_at timestamp with time zone,
    completed_at timestamp with time zone,
    expires_at timestamp with time zone NOT NULL,
    CONSTRAINT jobs_api_shape_check CHECK ((api_shape = ANY (ARRAY['granular'::text, 'unified'::text, 'any'::text, 'chat'::text]))),
    CONSTRAINT jobs_billing_mode_check CHECK ((billing_mode = ANY (ARRAY['fixed'::text, 'metered'::text]))),
    CONSTRAINT jobs_billing_state_check CHECK ((billing_state = ANY (ARRAY['pending_charge'::text, 'charged'::text, 'refunded_partial'::text, 'refunded_full'::text, 'not_charged'::text]))),
    CONSTRAINT jobs_canceled_by_check CHECK ((canceled_by = ANY (ARRAY['user'::text, 'system'::text, 'provider'::text]))),
    CONSTRAINT jobs_media_type_check CHECK ((media_type = ANY (ARRAY['image'::text, 'video'::text, 'audio'::text, 'mesh'::text, 'splat'::text, 'text'::text]))),
    CONSTRAINT jobs_price_unit_check CHECK ((price_unit = ANY (ARRAY['per_call'::text, 'per_image'::text, 'per_video_second'::text, 'per_audio_second'::text, 'per_megapixel'::text, 'per_second_compute'::text, 'per_million_tokens_in'::text, 'per_million_tokens_out'::text, 'per_mesh'::text, 'per_splat'::text]))),
    CONSTRAINT jobs_queue_stage_check CHECK ((queue_stage = ANY (ARRAY['at_our_gateway'::text, 'at_provider'::text]))),
    CONSTRAINT jobs_source_check CHECK ((source = ANY (ARRAY['api_key'::text, 'dashboard'::text, 'mcp'::text]))),
    CONSTRAINT jobs_status_check CHECK ((status = ANY (ARRAY['queued'::text, 'running'::text, 'succeeded'::text, 'failed'::text, 'canceled'::text, 'expired'::text])))
);


--
-- Name: jobs_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.jobs ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.jobs_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: ledger_entries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ledger_entries (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    job_id text,
    amount_micros bigint NOT NULL,
    entry_type text NOT NULL,
    stripe_payment_intent_id text,
    balance_after_micros bigint NOT NULL,
    memo text,
    idempotency_key text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT ledger_entries_entry_type_check CHECK ((entry_type = ANY (ARRAY['topup'::text, 'job_charge'::text, 'job_refund'::text, 'adjustment'::text, 'promo'::text, 'auto_refill'::text, 'chargeback'::text, 'chargeback_reversal'::text])))
);


--
-- Name: ledger_entries_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.ledger_entries ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.ledger_entries_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: legal_acceptances; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.legal_acceptances (
    sid bigint NOT NULL,
    id text NOT NULL,
    user_id text NOT NULL,
    account_id text,
    document text NOT NULL,
    version text NOT NULL,
    ip inet,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT legal_acceptances_document_check CHECK ((document = ANY (ARRAY['tos'::text, 'privacy'::text])))
);


--
-- Name: legal_acceptances_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.legal_acceptances ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.legal_acceptances_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: model_usage_summary; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.model_usage_summary (
    sid bigint NOT NULL,
    id text NOT NULL,
    model text NOT NULL,
    media_type text NOT NULL,
    model_owner text NOT NULL,
    window_days integer NOT NULL,
    generations bigint NOT NULL,
    output_units numeric NOT NULL,
    list_value_micros bigint NOT NULL,
    wow_generations numeric NOT NULL,
    rank_generations integer NOT NULL,
    rank_output integer NOT NULL,
    rank_value integer NOT NULL,
    sparkline integer[] NOT NULL,
    success_rate numeric,
    updated_at timestamp with time zone NOT NULL
);


--
-- Name: model_usage_summary_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.model_usage_summary ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.model_usage_summary_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: outputs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.outputs (
    sid bigint NOT NULL,
    id text NOT NULL,
    job_id text NOT NULL,
    file_id text,
    output_index integer NOT NULL,
    kind text NOT NULL,
    width integer,
    height integer,
    duration_ms integer,
    text_content text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT outputs_check CHECK (((file_id IS NOT NULL) OR (text_content IS NOT NULL))),
    CONSTRAINT outputs_kind_check CHECK ((kind = ANY (ARRAY['image'::text, 'video'::text, 'audio'::text, 'mesh'::text, 'splat'::text, 'text'::text])))
);


--
-- Name: outputs_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.outputs ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.outputs_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: password_resets; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.password_resets (
    sid bigint NOT NULL,
    id text NOT NULL,
    user_id text NOT NULL,
    token_hash text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    used_at timestamp with time zone
);


--
-- Name: password_resets_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.password_resets ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.password_resets_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: payment_methods; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.payment_methods (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    stripe_payment_method_id text NOT NULL,
    brand text,
    last4 text,
    exp_month integer,
    exp_year integer,
    is_default boolean NOT NULL,
    status text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT payment_methods_status_check CHECK ((status = ANY (ARRAY['active'::text, 'detached'::text])))
);


--
-- Name: payment_methods_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.payment_methods ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.payment_methods_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: provider_calls; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.provider_calls (
    sid bigint NOT NULL,
    id text NOT NULL,
    job_id text NOT NULL,
    provider text NOT NULL,
    direction text NOT NULL,
    url text,
    request_headers jsonb,
    request_body jsonb,
    is_request_truncated boolean NOT NULL,
    request_body_bytes bigint,
    response_status integer,
    response_body jsonb,
    is_response_truncated boolean NOT NULL,
    response_body_bytes bigint,
    latency_ms integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT provider_calls_direction_check CHECK ((direction = ANY (ARRAY['request'::text, 'response'::text, 'poll_request'::text, 'poll_response'::text, 'callback_in'::text])))
)
PARTITION BY RANGE (created_at);


--
-- Name: provider_calls_default; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.provider_calls_default (
    sid bigint NOT NULL,
    id text NOT NULL,
    job_id text NOT NULL,
    provider text NOT NULL,
    direction text NOT NULL,
    url text,
    request_headers jsonb,
    request_body jsonb,
    is_request_truncated boolean NOT NULL,
    request_body_bytes bigint,
    response_status integer,
    response_body jsonb,
    is_response_truncated boolean NOT NULL,
    response_body_bytes bigint,
    latency_ms integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT provider_calls_direction_check CHECK ((direction = ANY (ARRAY['request'::text, 'response'::text, 'poll_request'::text, 'poll_response'::text, 'callback_in'::text])))
);


--
-- Name: provider_calls_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.provider_calls ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.provider_calls_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: queue_tasks; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.queue_tasks (
    sid bigint NOT NULL,
    id text NOT NULL,
    task_type text NOT NULL,
    payload jsonb NOT NULL,
    priority integer NOT NULL,
    attempts integer NOT NULL,
    max_attempts integer NOT NULL,
    locked_by text,
    status text NOT NULL,
    last_error text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    run_at timestamp with time zone NOT NULL,
    locked_at timestamp with time zone,
    CONSTRAINT queue_tasks_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'running'::text, 'done'::text, 'dead'::text]))),
    CONSTRAINT queue_tasks_task_type_check CHECK ((task_type = ANY (ARRAY['dispatch_job'::text, 'poll_provider'::text, 'ingest_outputs'::text, 'deliver_webhook'::text, 'settle_refund'::text, 'auto_refill'::text, 'rollup_usage'::text, 'process_stripe_event'::text, 'fetch_asset'::text, 'send_email'::text, 'generate_invoice'::text])))
)
WITH (fillfactor='80');


--
-- Name: queue_tasks_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.queue_tasks ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.queue_tasks_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: request_logs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.request_logs (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text,
    api_key_id text,
    session_id text,
    job_id text,
    end_user_id text,
    source text NOT NULL,
    method text NOT NULL,
    path text NOT NULL,
    api_version date,
    ip inet,
    user_agent text,
    request_headers jsonb,
    request_body jsonb,
    is_request_truncated boolean NOT NULL,
    request_body_bytes bigint,
    response_status integer,
    response_body jsonb,
    is_response_truncated boolean NOT NULL,
    response_body_bytes bigint,
    latency_ms integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT request_logs_source_check CHECK ((source = ANY (ARRAY['api_key'::text, 'dashboard'::text, 'mcp'::text])))
)
PARTITION BY RANGE (created_at);


--
-- Name: request_logs_default; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.request_logs_default (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text,
    api_key_id text,
    session_id text,
    job_id text,
    end_user_id text,
    source text NOT NULL,
    method text NOT NULL,
    path text NOT NULL,
    api_version date,
    ip inet,
    user_agent text,
    request_headers jsonb,
    request_body jsonb,
    is_request_truncated boolean NOT NULL,
    request_body_bytes bigint,
    response_status integer,
    response_body jsonb,
    is_response_truncated boolean NOT NULL,
    response_body_bytes bigint,
    latency_ms integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT request_logs_source_check CHECK ((source = ANY (ARRAY['api_key'::text, 'dashboard'::text, 'mcp'::text])))
);


--
-- Name: request_logs_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.request_logs ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.request_logs_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: sessions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sessions (
    sid bigint NOT NULL,
    id text NOT NULL,
    user_id text NOT NULL,
    impersonator_user_id text,
    ip inet,
    user_agent text,
    device text,
    created_via text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    CONSTRAINT sessions_created_via_check CHECK ((created_via = ANY (ARRAY['signup'::text, 'invited_signup'::text, 'login'::text, 'sso_login'::text, 'sso_signup'::text, 'impersonation'::text])))
);


--
-- Name: sessions_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.sessions ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.sessions_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: spend_limits; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.spend_limits (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    created_by_user_id text NOT NULL,
    "window" text NOT NULL,
    limit_micros bigint NOT NULL,
    behavior text NOT NULL,
    period_start timestamp with time zone NOT NULL,
    spent_micros bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT spend_limits_behavior_check CHECK ((behavior = 'block'::text)),
    CONSTRAINT spend_limits_window_check CHECK (("window" = ANY (ARRAY['weekly'::text, 'monthly'::text])))
);


--
-- Name: spend_limits_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.spend_limits ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.spend_limits_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: stripe_customers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.stripe_customers (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    stripe_customer_id text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: stripe_customers_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.stripe_customers ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.stripe_customers_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: stripe_events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.stripe_events (
    sid bigint NOT NULL,
    id text NOT NULL,
    stripe_event_id text NOT NULL,
    event_type text NOT NULL,
    payload jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    processed_at timestamp with time zone,
    error text
);


--
-- Name: stripe_events_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.stripe_events ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.stripe_events_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: usage_rollups; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.usage_rollups (
    sid bigint NOT NULL,
    id text NOT NULL,
    grain text NOT NULL,
    period_start timestamp with time zone NOT NULL,
    media_type text NOT NULL,
    model text NOT NULL,
    model_owner text NOT NULL,
    modality text NOT NULL,
    provider text NOT NULL,
    source text NOT NULL,
    jobs_total bigint NOT NULL,
    jobs_succeeded bigint NOT NULL,
    jobs_failed bigint NOT NULL,
    output_units numeric NOT NULL,
    tokens_in bigint,
    tokens_out bigint,
    distinct_accounts integer NOT NULL,
    spend_micros bigint NOT NULL,
    p50_latency_ms integer,
    p95_latency_ms integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT usage_rollups_grain_check CHECK ((grain = ANY (ARRAY['hour'::text, 'day'::text, 'week'::text, 'month'::text]))),
    CONSTRAINT usage_rollups_media_type_check CHECK ((media_type = ANY (ARRAY['image'::text, 'video'::text, 'audio'::text, 'mesh'::text, 'splat'::text, 'text'::text])))
);


--
-- Name: usage_rollups_app; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.usage_rollups_app (
    sid bigint NOT NULL,
    id text NOT NULL,
    grain text NOT NULL,
    period_start timestamp with time zone NOT NULL,
    media_type text NOT NULL,
    model text NOT NULL,
    app_id text NOT NULL,
    jobs_total bigint NOT NULL,
    output_units numeric NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT usage_rollups_app_grain_check CHECK ((grain = ANY (ARRAY['hour'::text, 'day'::text, 'week'::text, 'month'::text])))
);


--
-- Name: usage_rollups_app_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.usage_rollups_app ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.usage_rollups_app_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: usage_rollups_category; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.usage_rollups_category (
    sid bigint NOT NULL,
    id text NOT NULL,
    grain text NOT NULL,
    period_start timestamp with time zone NOT NULL,
    media_type text NOT NULL,
    model text NOT NULL,
    model_owner text NOT NULL,
    content_category text NOT NULL,
    jobs_classified bigint NOT NULL,
    jobs_succeeded bigint NOT NULL,
    output_units numeric NOT NULL,
    distinct_accounts integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT usage_rollups_category_grain_check CHECK ((grain = ANY (ARRAY['hour'::text, 'day'::text, 'week'::text, 'month'::text])))
);


--
-- Name: usage_rollups_category_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.usage_rollups_category ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.usage_rollups_category_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: usage_rollups_language; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.usage_rollups_language (
    sid bigint NOT NULL,
    id text NOT NULL,
    grain text NOT NULL,
    period_start timestamp with time zone NOT NULL,
    media_type text NOT NULL,
    model text NOT NULL,
    model_owner text NOT NULL,
    language_kind text NOT NULL,
    language text NOT NULL,
    jobs_classified bigint NOT NULL,
    output_units numeric NOT NULL,
    distinct_accounts integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT usage_rollups_language_grain_check CHECK ((grain = ANY (ARRAY['hour'::text, 'day'::text, 'week'::text, 'month'::text]))),
    CONSTRAINT usage_rollups_language_language_kind_check CHECK ((language_kind = ANY (ARRAY['programming'::text, 'natural'::text]))),
    CONSTRAINT usage_rollups_language_media_type_check CHECK ((media_type = ANY (ARRAY['text'::text, 'audio'::text])))
);


--
-- Name: usage_rollups_language_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.usage_rollups_language ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.usage_rollups_language_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: usage_rollups_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.usage_rollups ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.usage_rollups_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: user_audit_logs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_audit_logs (
    sid bigint NOT NULL,
    id text NOT NULL,
    user_id text NOT NULL,
    actor_user_id text,
    impersonation_session_id text,
    event text NOT NULL,
    target_type text,
    target_id text,
    data jsonb,
    is_org_visible boolean NOT NULL,
    ip inet,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: user_audit_logs_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.user_audit_logs ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.user_audit_logs_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: user_avatar_files; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_avatar_files (
    sid bigint NOT NULL,
    id text NOT NULL,
    user_id text NOT NULL,
    original_user_avatar_file_id text,
    avatar_source text NOT NULL,
    bucket_hash text NOT NULL,
    content_hash text NOT NULL,
    bucket_suffix text,
    bucket_extension text NOT NULL,
    width integer,
    height integer,
    bytes bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT user_avatar_files_avatar_source_check CHECK ((avatar_source = ANY (ARRAY['custom_upload'::text, 'gravatar_mirror'::text])))
);


--
-- Name: user_avatar_files_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.user_avatar_files ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.user_avatar_files_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: user_avatars; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_avatars (
    sid bigint NOT NULL,
    id text NOT NULL,
    user_id text NOT NULL,
    custom_file_id text,
    gravatar_file_id text,
    preferred_source text DEFAULT 'not_yet_set'::text NOT NULL,
    gravatar_hash text,
    gravatar_checked_at timestamp with time zone,
    generated_seed text NOT NULL,
    email_first_letter text,
    email_last_letter text,
    person_name_first_letter text,
    person_name_last_letter text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT user_avatars_preferred_source_check CHECK ((preferred_source = ANY (ARRAY['not_yet_set'::text, 'custom'::text, 'gravatar'::text, 'letter'::text, 'generated'::text])))
);


--
-- Name: user_avatars_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.user_avatars ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.user_avatars_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: user_sso_connections; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_sso_connections (
    sid bigint NOT NULL,
    id text NOT NULL,
    user_id text NOT NULL,
    provider text NOT NULL,
    provider_user_id text NOT NULL,
    provider_email text NOT NULL,
    provider_user_display_name text,
    provider_user_first_name text,
    provider_user_last_name text,
    provider_username text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used_at timestamp with time zone NOT NULL,
    CONSTRAINT user_sso_connections_provider_check CHECK ((provider = ANY (ARRAY['google'::text, 'github'::text])))
);


--
-- Name: user_sso_connections_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.user_sso_connections ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.user_sso_connections_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: user_sso_disconnections; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_sso_disconnections (
    sid bigint NOT NULL,
    id text NOT NULL,
    user_id text NOT NULL,
    provider text NOT NULL,
    provider_user_id text NOT NULL,
    provider_email text NOT NULL,
    provider_user_display_name text,
    provider_user_first_name text,
    provider_user_last_name text,
    provider_username text,
    connected_at timestamp with time zone NOT NULL,
    disconnected_at timestamp with time zone NOT NULL,
    CONSTRAINT user_sso_disconnections_provider_check CHECK ((provider = ANY (ARRAY['google'::text, 'github'::text])))
);


--
-- Name: user_sso_disconnections_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.user_sso_disconnections ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.user_sso_disconnections_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.users (
    sid bigint NOT NULL,
    id text NOT NULL,
    email text NOT NULL,
    email_clean text NOT NULL,
    email_original text NOT NULL,
    email_verified_at timestamp with time zone,
    password_hash text,
    password_version integer DEFAULT 0 NOT NULL,
    session_version integer DEFAULT 0 NOT NULL,
    display_name text,
    totp_secret text,
    is_staff boolean DEFAULT false NOT NULL,
    tos_version text,
    tos_accepted_at timestamp with time zone,
    privacy_version text,
    privacy_accepted_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    deleted_at timestamp with time zone
);


--
-- Name: users_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.users ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.users_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: webhook_deliveries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.webhook_deliveries (
    sid bigint NOT NULL,
    id text NOT NULL,
    job_id text NOT NULL,
    account_id text NOT NULL,
    url text NOT NULL,
    event_type text NOT NULL,
    payload jsonb NOT NULL,
    attempt integer NOT NULL,
    response_status integer,
    response_body text,
    is_truncated boolean NOT NULL,
    latency_ms integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    next_retry_at timestamp with time zone,
    delivered_at timestamp with time zone,
    gave_up_at timestamp with time zone,
    CONSTRAINT webhook_deliveries_event_type_check CHECK ((event_type = ANY (ARRAY['job_completed'::text, 'job_failed'::text])))
)
PARTITION BY RANGE (created_at);


--
-- Name: webhook_deliveries_default; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.webhook_deliveries_default (
    sid bigint NOT NULL,
    id text NOT NULL,
    job_id text NOT NULL,
    account_id text NOT NULL,
    url text NOT NULL,
    event_type text NOT NULL,
    payload jsonb NOT NULL,
    attempt integer NOT NULL,
    response_status integer,
    response_body text,
    is_truncated boolean NOT NULL,
    latency_ms integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    next_retry_at timestamp with time zone,
    delivered_at timestamp with time zone,
    gave_up_at timestamp with time zone,
    CONSTRAINT webhook_deliveries_event_type_check CHECK ((event_type = ANY (ARRAY['job_completed'::text, 'job_failed'::text])))
);


--
-- Name: webhook_deliveries_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.webhook_deliveries ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.webhook_deliveries_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: webhook_secrets; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.webhook_secrets (
    sid bigint NOT NULL,
    id text NOT NULL,
    account_id text NOT NULL,
    secret text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    revoked_at timestamp with time zone
);


--
-- Name: webhook_secrets_sid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

ALTER TABLE public.webhook_secrets ALTER COLUMN sid ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.webhook_secrets_sid_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: provider_calls_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.provider_calls ATTACH PARTITION public.provider_calls_default DEFAULT;


--
-- Name: request_logs_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.request_logs ATTACH PARTITION public.request_logs_default DEFAULT;


--
-- Name: webhook_deliveries_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.webhook_deliveries ATTACH PARTITION public.webhook_deliveries_default DEFAULT;


--
-- Name: account_audit_logs account_audit_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.account_audit_logs
    ADD CONSTRAINT account_audit_logs_pkey PRIMARY KEY (id);


--
-- Name: account_balances account_balances_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.account_balances
    ADD CONSTRAINT account_balances_pkey PRIMARY KEY (account_id);


--
-- Name: account_members account_members_account_id_user_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.account_members
    ADD CONSTRAINT account_members_account_id_user_id_key UNIQUE (account_id, user_id);


--
-- Name: account_members account_members_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.account_members
    ADD CONSTRAINT account_members_pkey PRIMARY KEY (id);


--
-- Name: accounts accounts_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.accounts
    ADD CONSTRAINT accounts_pkey PRIMARY KEY (id);


--
-- Name: api_keys api_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_pkey PRIMARY KEY (id);


--
-- Name: asset_ingests asset_ingests_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.asset_ingests
    ADD CONSTRAINT asset_ingests_pkey PRIMARY KEY (id);


--
-- Name: auto_refill_rules auto_refill_rules_account_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.auto_refill_rules
    ADD CONSTRAINT auto_refill_rules_account_id_key UNIQUE (account_id);


--
-- Name: auto_refill_rules auto_refill_rules_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.auto_refill_rules
    ADD CONSTRAINT auto_refill_rules_pkey PRIMARY KEY (id);


--
-- Name: billing_events billing_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.billing_events
    ADD CONSTRAINT billing_events_pkey PRIMARY KEY (id);


--
-- Name: content_labels content_labels_job_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.content_labels
    ADD CONSTRAINT content_labels_job_id_key UNIQUE (job_id);


--
-- Name: content_labels content_labels_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.content_labels
    ADD CONSTRAINT content_labels_pkey PRIMARY KEY (id);


--
-- Name: email_changes email_changes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.email_changes
    ADD CONSTRAINT email_changes_pkey PRIMARY KEY (id);


--
-- Name: email_sends email_sends_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.email_sends
    ADD CONSTRAINT email_sends_pkey PRIMARY KEY (id);


--
-- Name: email_verifications email_verifications_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.email_verifications
    ADD CONSTRAINT email_verifications_pkey PRIMARY KEY (id);


--
-- Name: end_users end_users_account_id_external_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.end_users
    ADD CONSTRAINT end_users_account_id_external_id_key UNIQUE (account_id, external_id);


--
-- Name: end_users end_users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.end_users
    ADD CONSTRAINT end_users_pkey PRIMARY KEY (id);


--
-- Name: files files_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.files
    ADD CONSTRAINT files_pkey PRIMARY KEY (id);


--
-- Name: idempotency_keys idempotency_keys_account_id_key_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.idempotency_keys
    ADD CONSTRAINT idempotency_keys_account_id_key_key UNIQUE (account_id, key);


--
-- Name: idempotency_keys idempotency_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.idempotency_keys
    ADD CONSTRAINT idempotency_keys_pkey PRIMARY KEY (id);


--
-- Name: impersonation_audit_logs impersonation_audit_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.impersonation_audit_logs
    ADD CONSTRAINT impersonation_audit_logs_pkey PRIMARY KEY (id);


--
-- Name: invitations invitations_code_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.invitations
    ADD CONSTRAINT invitations_code_key UNIQUE (code);


--
-- Name: invitations invitations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.invitations
    ADD CONSTRAINT invitations_pkey PRIMARY KEY (id);


--
-- Name: invoice_counters invoice_counters_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.invoice_counters
    ADD CONSTRAINT invoice_counters_pkey PRIMARY KEY (scope);


--
-- Name: invoices invoices_account_id_period_start_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_account_id_period_start_key UNIQUE (account_id, period_start);


--
-- Name: invoices invoices_number_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_number_key UNIQUE (number);


--
-- Name: invoices invoices_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_pkey PRIMARY KEY (id);


--
-- Name: job_events job_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.job_events
    ADD CONSTRAINT job_events_pkey PRIMARY KEY (id);


--
-- Name: jobs jobs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.jobs
    ADD CONSTRAINT jobs_pkey PRIMARY KEY (id);


--
-- Name: ledger_entries ledger_entries_idempotency_key_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ledger_entries
    ADD CONSTRAINT ledger_entries_idempotency_key_key UNIQUE (idempotency_key);


--
-- Name: ledger_entries ledger_entries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ledger_entries
    ADD CONSTRAINT ledger_entries_pkey PRIMARY KEY (id);


--
-- Name: legal_acceptances legal_acceptances_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.legal_acceptances
    ADD CONSTRAINT legal_acceptances_pkey PRIMARY KEY (id);


--
-- Name: model_usage_summary model_usage_summary_model_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.model_usage_summary
    ADD CONSTRAINT model_usage_summary_model_key UNIQUE (model);


--
-- Name: model_usage_summary model_usage_summary_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.model_usage_summary
    ADD CONSTRAINT model_usage_summary_pkey PRIMARY KEY (id);


--
-- Name: outputs outputs_job_id_output_index_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.outputs
    ADD CONSTRAINT outputs_job_id_output_index_key UNIQUE (job_id, output_index);


--
-- Name: outputs outputs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.outputs
    ADD CONSTRAINT outputs_pkey PRIMARY KEY (id);


--
-- Name: password_resets password_resets_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.password_resets
    ADD CONSTRAINT password_resets_pkey PRIMARY KEY (id);


--
-- Name: payment_methods payment_methods_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.payment_methods
    ADD CONSTRAINT payment_methods_pkey PRIMARY KEY (id);


--
-- Name: provider_calls provider_calls_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.provider_calls
    ADD CONSTRAINT provider_calls_pkey PRIMARY KEY (id, created_at);


--
-- Name: provider_calls_default provider_calls_default_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.provider_calls_default
    ADD CONSTRAINT provider_calls_default_pkey PRIMARY KEY (id, created_at);


--
-- Name: queue_tasks queue_tasks_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.queue_tasks
    ADD CONSTRAINT queue_tasks_pkey PRIMARY KEY (id);


--
-- Name: request_logs request_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.request_logs
    ADD CONSTRAINT request_logs_pkey PRIMARY KEY (id, created_at);


--
-- Name: request_logs_default request_logs_default_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.request_logs_default
    ADD CONSTRAINT request_logs_default_pkey PRIMARY KEY (id, created_at);


--
-- Name: sessions sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sessions
    ADD CONSTRAINT sessions_pkey PRIMARY KEY (id);


--
-- Name: spend_limits spend_limits_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.spend_limits
    ADD CONSTRAINT spend_limits_pkey PRIMARY KEY (id);


--
-- Name: stripe_customers stripe_customers_account_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.stripe_customers
    ADD CONSTRAINT stripe_customers_account_id_key UNIQUE (account_id);


--
-- Name: stripe_customers stripe_customers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.stripe_customers
    ADD CONSTRAINT stripe_customers_pkey PRIMARY KEY (id);


--
-- Name: stripe_customers stripe_customers_stripe_customer_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.stripe_customers
    ADD CONSTRAINT stripe_customers_stripe_customer_id_key UNIQUE (stripe_customer_id);


--
-- Name: stripe_events stripe_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.stripe_events
    ADD CONSTRAINT stripe_events_pkey PRIMARY KEY (id);


--
-- Name: stripe_events stripe_events_stripe_event_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.stripe_events
    ADD CONSTRAINT stripe_events_stripe_event_id_key UNIQUE (stripe_event_id);


--
-- Name: usage_rollups_app usage_rollups_app_grain_period_start_media_type_model_app_i_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.usage_rollups_app
    ADD CONSTRAINT usage_rollups_app_grain_period_start_media_type_model_app_i_key UNIQUE (grain, period_start, media_type, model, app_id);


--
-- Name: usage_rollups_app usage_rollups_app_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.usage_rollups_app
    ADD CONSTRAINT usage_rollups_app_pkey PRIMARY KEY (id);


--
-- Name: usage_rollups_category usage_rollups_category_grain_period_start_media_type_model__key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.usage_rollups_category
    ADD CONSTRAINT usage_rollups_category_grain_period_start_media_type_model__key UNIQUE (grain, period_start, media_type, model, content_category);


--
-- Name: usage_rollups_category usage_rollups_category_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.usage_rollups_category
    ADD CONSTRAINT usage_rollups_category_pkey PRIMARY KEY (id);


--
-- Name: usage_rollups usage_rollups_grain_period_start_media_type_model_modality__key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.usage_rollups
    ADD CONSTRAINT usage_rollups_grain_period_start_media_type_model_modality__key UNIQUE (grain, period_start, media_type, model, modality, provider, source);


--
-- Name: usage_rollups_language usage_rollups_language_grain_period_start_media_type_model__key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.usage_rollups_language
    ADD CONSTRAINT usage_rollups_language_grain_period_start_media_type_model__key UNIQUE (grain, period_start, media_type, model, language_kind, language);


--
-- Name: usage_rollups_language usage_rollups_language_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.usage_rollups_language
    ADD CONSTRAINT usage_rollups_language_pkey PRIMARY KEY (id);


--
-- Name: usage_rollups usage_rollups_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.usage_rollups
    ADD CONSTRAINT usage_rollups_pkey PRIMARY KEY (id);


--
-- Name: user_audit_logs user_audit_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_audit_logs
    ADD CONSTRAINT user_audit_logs_pkey PRIMARY KEY (id);


--
-- Name: user_avatar_files user_avatar_files_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_avatar_files
    ADD CONSTRAINT user_avatar_files_pkey PRIMARY KEY (id);


--
-- Name: user_avatars user_avatars_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_avatars
    ADD CONSTRAINT user_avatars_pkey PRIMARY KEY (id);


--
-- Name: user_avatars user_avatars_user_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_avatars
    ADD CONSTRAINT user_avatars_user_id_key UNIQUE (user_id);


--
-- Name: user_sso_connections user_sso_connections_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_sso_connections
    ADD CONSTRAINT user_sso_connections_pkey PRIMARY KEY (id);


--
-- Name: user_sso_connections user_sso_connections_provider_provider_user_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_sso_connections
    ADD CONSTRAINT user_sso_connections_provider_provider_user_id_key UNIQUE (provider, provider_user_id);


--
-- Name: user_sso_connections user_sso_connections_user_id_provider_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_sso_connections
    ADD CONSTRAINT user_sso_connections_user_id_provider_key UNIQUE (user_id, provider);


--
-- Name: user_sso_disconnections user_sso_disconnections_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_sso_disconnections
    ADD CONSTRAINT user_sso_disconnections_pkey PRIMARY KEY (id);


--
-- Name: users users_email_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_key UNIQUE (email);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: webhook_deliveries webhook_deliveries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.webhook_deliveries
    ADD CONSTRAINT webhook_deliveries_pkey PRIMARY KEY (id, created_at);


--
-- Name: webhook_deliveries_default webhook_deliveries_default_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.webhook_deliveries_default
    ADD CONSTRAINT webhook_deliveries_default_pkey PRIMARY KEY (id, created_at);


--
-- Name: webhook_secrets webhook_secrets_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.webhook_secrets
    ADD CONSTRAINT webhook_secrets_pkey PRIMARY KEY (id);


--
-- Name: account_audit_logs_account_created_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX account_audit_logs_account_created_idx ON public.account_audit_logs USING btree (account_id, created_at DESC);


--
-- Name: account_members_one_owner; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX account_members_one_owner ON public.account_members USING btree (account_id) WHERE (role = 'owner'::text);


--
-- Name: asset_ingests_account_created_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX asset_ingests_account_created_idx ON public.asset_ingests USING btree (account_id, created_at DESC);


--
-- Name: asset_ingests_source_url_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX asset_ingests_source_url_idx ON public.asset_ingests USING btree (source_url);


--
-- Name: billing_events_account_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX billing_events_account_created ON public.billing_events USING btree (account_id, created_at DESC);


--
-- Name: billing_events_job; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX billing_events_job ON public.billing_events USING btree (job_id);


--
-- Name: invitations_one_live_per_account_email; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX invitations_one_live_per_account_email ON public.invitations USING btree (account_id, email) WHERE ((accepted_at IS NULL) AND (revoked_at IS NULL));


--
-- Name: invoices_account_period; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX invoices_account_period ON public.invoices USING btree (account_id, period_start DESC);


--
-- Name: jobs_account_created_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX jobs_account_created_idx ON public.jobs USING btree (account_id, created_at DESC);


--
-- Name: jobs_account_enduser_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX jobs_account_enduser_idx ON public.jobs USING btree (account_id, end_user_id, created_at) WHERE (end_user_id IS NOT NULL);


--
-- Name: jobs_account_status_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX jobs_account_status_idx ON public.jobs USING btree (account_id, status);


--
-- Name: jobs_provider_jobid_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX jobs_provider_jobid_idx ON public.jobs USING btree (provider, provider_job_id);


--
-- Name: jobs_status_expires_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX jobs_status_expires_idx ON public.jobs USING btree (status, expires_at);


--
-- Name: legal_acceptances_account_doc_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX legal_acceptances_account_doc_idx ON public.legal_acceptances USING btree (account_id, document, created_at DESC);


--
-- Name: legal_acceptances_user_doc_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX legal_acceptances_user_doc_idx ON public.legal_acceptances USING btree (user_id, document, created_at DESC);


--
-- Name: payment_methods_one_default; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX payment_methods_one_default ON public.payment_methods USING btree (account_id) WHERE (is_default AND (status = 'active'::text));


--
-- Name: provider_calls_job_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX provider_calls_job_idx ON ONLY public.provider_calls USING btree (job_id);


--
-- Name: provider_calls_default_job_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX provider_calls_default_job_id_idx ON public.provider_calls_default USING btree (job_id);


--
-- Name: queue_tasks_worker_loop_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX queue_tasks_worker_loop_idx ON public.queue_tasks USING btree (status, run_at, priority);


--
-- Name: request_logs_account_created_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX request_logs_account_created_idx ON ONLY public.request_logs USING btree (account_id, created_at);


--
-- Name: request_logs_default_account_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX request_logs_default_account_id_created_at_idx ON public.request_logs_default USING btree (account_id, created_at);


--
-- Name: request_logs_job_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX request_logs_job_idx ON ONLY public.request_logs USING btree (job_id);


--
-- Name: request_logs_default_job_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX request_logs_default_job_id_idx ON public.request_logs_default USING btree (job_id);


--
-- Name: user_audit_logs_user_created_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_audit_logs_user_created_idx ON public.user_audit_logs USING btree (user_id, created_at DESC);


--
-- Name: user_audit_logs_user_created_orgvis_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_audit_logs_user_created_orgvis_idx ON public.user_audit_logs USING btree (user_id, created_at DESC) WHERE is_org_visible;


--
-- Name: webhook_deliveries_account_created_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX webhook_deliveries_account_created_idx ON ONLY public.webhook_deliveries USING btree (account_id, created_at);


--
-- Name: webhook_deliveries_default_account_id_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX webhook_deliveries_default_account_id_created_at_idx ON public.webhook_deliveries_default USING btree (account_id, created_at);


--
-- Name: webhook_deliveries_job_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX webhook_deliveries_job_idx ON ONLY public.webhook_deliveries USING btree (job_id);


--
-- Name: webhook_deliveries_default_job_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX webhook_deliveries_default_job_id_idx ON public.webhook_deliveries_default USING btree (job_id);


--
-- Name: provider_calls_default_job_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.provider_calls_job_idx ATTACH PARTITION public.provider_calls_default_job_id_idx;


--
-- Name: provider_calls_default_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.provider_calls_pkey ATTACH PARTITION public.provider_calls_default_pkey;


--
-- Name: request_logs_default_account_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.request_logs_account_created_idx ATTACH PARTITION public.request_logs_default_account_id_created_at_idx;


--
-- Name: request_logs_default_job_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.request_logs_job_idx ATTACH PARTITION public.request_logs_default_job_id_idx;


--
-- Name: request_logs_default_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.request_logs_pkey ATTACH PARTITION public.request_logs_default_pkey;


--
-- Name: webhook_deliveries_default_account_id_created_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.webhook_deliveries_account_created_idx ATTACH PARTITION public.webhook_deliveries_default_account_id_created_at_idx;


--
-- Name: webhook_deliveries_default_job_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.webhook_deliveries_job_idx ATTACH PARTITION public.webhook_deliveries_default_job_id_idx;


--
-- Name: webhook_deliveries_default_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.webhook_deliveries_pkey ATTACH PARTITION public.webhook_deliveries_default_pkey;


--
-- PostgreSQL database dump complete
--

\unrestrict 6QFBWPdccQH2gTTBPNMNcbgRmZLI7QjKnz1t1PklvF6laO4SjONywTtog9A8SGm

