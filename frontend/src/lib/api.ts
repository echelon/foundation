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

async function send<T>(method: string, path: string, init: { headers?: Record<string, string>; body?: BodyInit }): Promise<T> {
  let resp: Response;
  try {
    resp = await fetch(`${API_BASE_URL}${path}`, {
      method,
      credentials: "include",
      headers: init.headers,
      body: init.body,
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

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  return send(
    method,
    path,
    body !== undefined
      ? { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }
      : {},
  );
}

/* ── Types (mirror the gateway's JSON) ── */

/** CDN links to an avatar image. `thumbnail_template` carries a `{WIDTH}` placeholder (in dev
 * it's just the full-image URL — no resizer in front of the bucket). */
export type AvatarLinks = {
  cdn_url: string;
  thumbnail_template: string;
};

/** What a user can choose to display. */
export type AvatarSource = "custom" | "gravatar" | "letter";

/** `preferred_source` values: a choice, or `not_yet_set` (system auto-picks: gravatar when
 * mirrored, else the monogram). */
export type PreferredSource = AvatarSource | "not_yet_set" | "generated";

/** The avatar as APIs report it: the user's preference, the derived effective source (what
 * actually displays), CDN links (null → render the monogram), and availability flags. */
export type ApiAvatar = {
  /** What's actually displaying: the preference when satisfiable, else "letter". */
  source: string;
  /** The user's stated choice; "not_yet_set" until they pick (system auto-picks meanwhile). */
  preferred_source: PreferredSource;
  links: AvatarLinks | null;
  /** A custom upload exists (choosable even when not displaying). */
  has_custom: boolean;
  /** A mirrored gravatar exists (choosable even when not displaying). */
  has_gravatar: boolean;
  /** False while a gravatar (re-)poll is pending — the background job hasn't checked yet. */
  gravatar_checked: boolean;
};

/** Crop rect in source-image pixels (square — encodes zoom + translate of a square viewport). */
export type AvatarCrop = { x: number; y: number; size: number };

export type ApiUser = {
  id: string;
  email: string;
  display_name: string | null;
  is_staff: boolean;
  email_verified: boolean;
  /** Only present on `/me` responses; absent right after login/signup (refreshed on next load). */
  avatar?: ApiAvatar;
};

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

/** One of the caller's organizations (memberships), for the org switcher. */
export type ApiAccount = {
  account_id: string;
  name: string | null;
  role: string;
  joined_at: string;
};

/** A row in an organization's member roster. */
export type ApiMember = {
  user_id: string;
  display_name: string | null;
  email: string;
  role: string;
  joined_at: string;
  is_you: boolean;
};

/** One audit-log row (user or account trail). */
export type AuditItem = {
  id: string;
  event: string;
  actor_user_id: string | null;
  user_id?: string | null;
  target_type: string | null;
  target_id: string | null;
  data: Record<string, unknown> | null;
  is_org_visible?: boolean;
  ip: string | null;
  created_at: string;
};

/** A keyset-paginated page: pass `next_before` back as `before` for the next page. */
export type AuditPage = { items: AuditItem[]; next_before: string | null };

function pageQuery(before?: string, limit = 25): string {
  const params = new URLSearchParams({ limit: String(limit) });
  if (before) params.set("before", before);
  return `?${params}`;
}

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

  /** Send (or re-send) the verification email for the current address. */
  requestEmailVerification: () =>
    request<{ ok: boolean; message: string }>("POST", "/v1/auth/email/verification"),

  /** Consume a verification token (from the emailed link). Writes the org-visible
   * `email_verified` audit event. */
  confirmEmailVerification: (token: string) =>
    request<{ ok: boolean }>("POST", "/v1/auth/email/verification/confirm", { token }),

  /** Update the display name (empty/undefined clears it). Writes the org-visible
   * `display_name_changed` audit event. */
  updateProfile: (displayName: string | null) =>
    request<{ ok: boolean; display_name: string | null }>("PUT", "/v1/auth/profile", {
      display_name: displayName,
    }),

  /* ── Avatar ── */

  /** Upload/replace the avatar. Body is the raw file; `crop` (source pixels) encodes the
   * square viewport's zoom + pan — omitted, the server center-crops the largest square. */
  uploadAvatar: (file: Blob, crop?: AvatarCrop) => {
    const params = crop ? `?crop_x=${crop.x}&crop_y=${crop.y}&crop_size=${crop.size}` : "";
    return send<{ ok: boolean; avatar: ApiAvatar }>("POST", `/v1/auth/avatar${params}`, { body: file });
  },

  removeAvatar: () => request<{ ok: boolean; avatar: ApiAvatar }>("DELETE", "/v1/auth/avatar"),

  /** Choose what displays: photo, gravatar, or the letter monogram. */
  setAvatarPreference: (source: AvatarSource) =>
    request<{ ok: boolean; avatar: ApiAvatar }>("PUT", "/v1/auth/avatar/preference", { source }),

  /** Remove the mirrored gravatar (won't re-mirror unless explicitly refreshed). */
  removeGravatar: () => request<{ ok: boolean; avatar: ApiAvatar }>("DELETE", "/v1/auth/avatar/gravatar"),

  /** Re-request the gravatar — the background job re-fetches within seconds. */
  refreshGravatar: () => request<{ ok: boolean; avatar: ApiAvatar }>("POST", "/v1/auth/avatar/gravatar/refresh"),

  /* ── Organizations ── */

  /** Every organization the caller belongs to (drives the org switcher). */
  listAccounts: () => request<{ accounts: ApiAccount[] }>("GET", "/v1/accounts"),

  /** The member roster of an organization (any member may view). */
  listMembers: (accountId: string) =>
    request<{ members: ApiMember[]; your_role: string }>("GET", `/v1/accounts/${accountId}/members`),

  /** Invite someone (owner/admin). Returns the code + join link to hand to the invitee. */
  inviteMember: (accountId: string, email: string, role: "admin" | "member") =>
    request<{ invitation_id: string; code: string; join_url: string; email: string; role: string; expires_at: string }>(
      "POST",
      `/v1/accounts/${accountId}/invitations`,
      { email, role },
    ),

  /** Remove a member (owner/admin; owner is immovable, admins can't remove admins). */
  removeMember: (accountId: string, userId: string) =>
    request<{ ok: boolean }>("DELETE", `/v1/accounts/${accountId}/members/${userId}`),

  /* ── Audit logs (keyset-paginated) ── */

  /** The caller's own audit trail (private rows included). */
  userAuditLogs: (before?: string, limit?: number) =>
    request<AuditPage>("GET", `/v1/user/audit_logs${pageQuery(before, limit)}`),

  /** The organization's audit trail (owner/admin only). */
  accountAuditLogs: (accountId: string, before?: string, limit?: number) =>
    request<AuditPage>("GET", `/v1/accounts/${accountId}/audit_logs${pageQuery(before, limit)}`),

  /** A member's trail as the org sees it — org-visible rows only (owner/admin only). */
  memberAuditLogs: (accountId: string, userId: string, before?: string, limit?: number) =>
    request<AuditPage>("GET", `/v1/accounts/${accountId}/members/${userId}/audit_logs${pageQuery(before, limit)}`),

  /* ── Sessions ── */

  listSessions: () => request<{ sessions: ApiSession[] }>("GET", "/v1/auth/sessions"),

  revokeSession: (id: string) => request<{ ok: boolean }>("DELETE", `/v1/auth/sessions/${id}`),

  revokeOtherSessions: () => request<{ ok: boolean }>("DELETE", "/v1/auth/sessions?others=true"),
};
