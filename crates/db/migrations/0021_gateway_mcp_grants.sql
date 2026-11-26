CREATE TABLE gateway_mcp_grants (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    session_version INTEGER NOT NULL,
    account_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    client_ref TEXT,
    oauth_client_id TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    resource TEXT NOT NULL,
    scopes TEXT[] NOT NULL,
    code_challenge TEXT NOT NULL,
    code_hash TEXT UNIQUE,
    code_expires_at TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '5 minutes',
    access_hash TEXT UNIQUE,
    access_expires_at TIMESTAMPTZ,
    refresh_hash TEXT UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '30 days',
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (scopes <@ ARRAY['requests:read', 'requests:submit', 'requests:cancel', 'usage:read']::TEXT[])
);

CREATE INDEX gateway_mcp_grants_user_created_idx ON gateway_mcp_grants (user_id, created_at DESC);
