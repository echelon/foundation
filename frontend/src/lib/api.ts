/**
 * The real API client (replaces mock-api for auth/account surfaces).
 *
 * ── Which server does the frontend hit? ──────────────────────────────────────
 * Decided ONCE at startup, from Vite env:
 *   npm run dev        → local api-gateway  (http://localhost:4000)
 *   npm run dev:prod   → production server  (https://api.foundation.ai)
 * An explicit VITE_API_BASE_URL overrides both (e.g. a staging box).
 *
 * All requests send credentials (the `foundation_session` HttpOnly cookie);
 * the gateway's CORS layer allows the dev origins.
 */

const PRODUCTION_API_URL = "https://api.foundation.ai";
const LOCAL_API_URL = "http://localhost:4000";

function resolveApiBaseUrl(): string {
  const explicit = import.meta.env.VITE_API_BASE_URL;
  if (explicit) return explicit.replace(/\/+$/, "");
  return import.meta.env.VITE_API_TARGET === "production" ? PRODUCTION_API_URL : LOCAL_API_URL;
}

export const API_BASE_URL = resolveApiBaseUrl();

/** The `{ error: { code, message } }` envelope the gateway returns on failure. */
export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  let resp: Response;
  try {
    resp = await fetch(`${API_BASE_URL}${path}`, {
      method,
      credentials: "include",
      headers: body !== undefined ? { "Content-Type": "application/json" } : undefined,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
  } catch {
    throw new ApiError(0, "network", `Could not reach the API at ${API_BASE_URL}. Is the server running?`);
  }
  const json = await resp.json().catch(() => null);
  if (!resp.ok) {
    const err = (json as { error?: { code?: string; message?: string } } | null)?.error;
    throw new ApiError(resp.status, err?.code ?? "unknown", err?.message ?? `Request failed (${resp.status})`);
  }
  return json as T;
}

/* ── Types (mirror the gateway's JSON) ── */

export type ApiUser = {
  id: string;
  email: string;
  display_name: string | null;
  is_staff: boolean;
  email_verified: boolean;
  /** Avatar display source (`custom` / `gravatar` / `letter` / `generated`). Only present on
   * `/me` responses; absent right after login/signup (refreshed on next load). */
  avatar_source?: string;
};

/** The caller's avatar image (gravatar proxied through the gateway; 404 → use the monogram). */
export const AVATAR_URL = `${API_BASE_URL}/v1/auth/avatar`;

export type ApiSession = {
  id: string;
  ip: string | null;
  user_agent: string | null;
  device: string | null;
  created_at: string;
  expires_at: string;
  this_device: boolean;
};

type AuthResponse = { user: ApiUser; session_id: string };
type MeResponse = { user: ApiUser; session_id: string; impersonation: boolean };

/* ── Auth ── */

export const api = {
  signup: (email: string, password: string, displayName?: string) =>
    request<AuthResponse>("POST", "/v1/auth/signup", {
      email,
      password,
      display_name: displayName || undefined,
    }),

  login: (email: string, password: string) =>
    request<AuthResponse>("POST", "/v1/auth/login", { email, password }),

  logout: () => request<{ ok: boolean }>("POST", "/v1/auth/logout"),

  me: () => request<MeResponse>("GET", "/v1/auth/me"),

  /* ── Account security ── */

  changePassword: (currentPassword: string, newPassword: string, logoutOtherSessions: boolean) =>
    request<{ ok: boolean }>("PUT", "/v1/auth/password", {
      current_password: currentPassword,
      new_password: newPassword,
      logout_other_sessions: logoutOtherSessions,
    }),

  changeEmail: (newEmail: string, currentPassword: string) =>
    request<{ ok: boolean; email: string; email_verified: boolean }>("PUT", "/v1/auth/email", {
      new_email: newEmail,
      current_password: currentPassword,
    }),

  requestPasswordReset: (email: string) =>
    request<{ ok: boolean; message: string }>("POST", "/v1/auth/password-reset", { email }),

  confirmPasswordReset: (token: string, newPassword: string, logoutOtherSessions: boolean) =>
    request<{ ok: boolean }>("POST", "/v1/auth/password-reset/confirm", {
      token,
      new_password: newPassword,
      logout_other_sessions: logoutOtherSessions,
    }),

  /* ── Sessions ── */

  listSessions: () => request<{ sessions: ApiSession[] }>("GET", "/v1/auth/sessions"),

  revokeSession: (id: string) => request<{ ok: boolean }>("DELETE", `/v1/auth/sessions/${id}`),

  revokeOtherSessions: () => request<{ ok: boolean }>("DELETE", "/v1/auth/sessions?others=true"),
};
