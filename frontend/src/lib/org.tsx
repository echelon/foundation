import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { api, type ApiAccount } from "./api";
import { useAuth } from "./auth";

/**
 * Organization scope for the dashboard, driven by the URL.
 *
 * Multi-org users get account-scoped URLs — `/dashboard/{account_id}/requests` — so links
 * are shareable and the switcher just navigates to the same page under another account.
 * Single-org users keep unscoped URLs (`/dashboard/requests`). User-scoped pages
 * (`/dashboard/user/…`) never carry an account id, but the selected scope persists across
 * them.
 *
 * A trivial unencrypted cookie remembers the last-viewed account so `/dashboard` (no account
 * in the URL) redirects a multi-org user back to where they were.
 */

const LAST_ACCOUNT_COOKIE = "foundation_last_account";

/** Remember the last-viewed account (plain cookie — a UI hint, not a credential). */
export function rememberLastAccount(accountId: string) {
  document.cookie = `${LAST_ACCOUNT_COOKIE}=${accountId}; path=/; max-age=31536000; samesite=lax`;
}

function readLastAccount(): string | null {
  for (const pair of document.cookie.split(";")) {
    const [k, v] = pair.trim().split("=");
    if (k === LAST_ACCOUNT_COOKIE && v) return v;
  }
  return null;
}

/** The `acct_…` segment of an account-scoped dashboard URL, if present. */
function accountIdInPath(pathname: string): string | null {
  const m = pathname.match(/^\/dashboard\/(acct_[A-Za-z0-9]+)(?:\/|$)/);
  return m ? m[1] : null;
}

/** The page part of an account-scoped dashboard URL ("" for overview, "/requests", …), or
 * null when the location isn't account-scoped (e.g. `/dashboard/user/settings`). */
function scopedRest(pathname: string): string | null {
  if (pathname === "/dashboard" || pathname === "/dashboard/") return "";
  const withId = pathname.match(/^\/dashboard\/acct_[A-Za-z0-9]+(\/.*)?$/);
  if (withId) return withId[1] ?? "";
  const bare = pathname.match(/^\/dashboard(\/.*)$/);
  if (!bare) return null;
  return bare[1].startsWith("/user/") || bare[1] === "/user" ? null : bare[1];
}

type OrgContextValue = {
  accounts: ApiAccount[];
  /** The currently-selected organization (null while loading / if none). */
  current: ApiAccount | null;
  /** Switch scope: updates the URL (staying on the same page) and the cookie. */
  select: (accountId: string) => void;
  /** Build a dashboard href under the current scope, e.g. `withScope("/requests")` →
   * `/dashboard/acct_…/requests` for multi-org users, `/dashboard/requests` otherwise. */
  withScope: (rest: string) => string;
  reload: () => Promise<void>;
  loading: boolean;
};

const OrgCtx = createContext<OrgContextValue>({
  accounts: [],
  current: null,
  select: () => {},
  withScope: (rest) => `/dashboard${rest}`,
  reload: async () => {},
  loading: true,
});

export function OrgProvider({ children }: { children: ReactNode }) {
  const { user } = useAuth();
  const location = useLocation();
  const navigate = useNavigate();
  const [accounts, setAccounts] = useState<ApiAccount[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    if (!user) return;
    try {
      const r = await api.listAccounts();
      setAccounts(r.accounts);
    } catch {
      setAccounts([]);
    } finally {
      setLoading(false);
    }
  }, [user]);

  useEffect(() => {
    setLoading(true);
    void reload();
  }, [reload]);

  const urlAccountId = accountIdInPath(location.pathname);

  // Resolve the scope: the URL wins, then an in-session selection, then the cookie, then the
  // oldest membership (the signup-created account).
  const current = useMemo(() => {
    if (accounts.length === 0) return null;
    const byId = (id: string | null) => (id ? accounts.find((a) => a.account_id === id) : undefined);
    return byId(urlAccountId) ?? byId(selectedId) ?? byId(readLastAccount()) ?? accounts[0];
  }, [accounts, urlAccountId, selectedId]);

  const multi = accounts.length > 1;

  const withScope = useCallback(
    (rest: string) => (multi && current ? `/dashboard/${current.account_id}${rest}` : `/dashboard${rest}`),
    [multi, current],
  );

  // Remember the scope whenever it resolves.
  useEffect(() => {
    if (current) rememberLastAccount(current.account_id);
  }, [current]);

  // Canonicalize the URL once memberships are known:
  //   - multi-org + unscoped account page → inject the remembered scope,
  //   - single-org + scoped page → drop the id (single-org URLs stay clean),
  //   - an id we're not a member of → back to the scope that matters.
  useEffect(() => {
    if (loading || !current) return;
    const rest = scopedRest(location.pathname);
    if (rest === null) return; // user-scoped page — no account in these URLs
    const canonical = multi ? `/dashboard/${current.account_id}${rest}` : `/dashboard${rest}`;
    if (location.pathname !== canonical) navigate(canonical + location.search, { replace: true });
  }, [loading, current, multi, location.pathname, location.search, navigate]);

  const select = useCallback(
    (accountId: string) => {
      setSelectedId(accountId);
      rememberLastAccount(accountId);
      // On an account-scoped page, jump to the same page under the new scope.
      const rest = scopedRest(location.pathname);
      if (rest !== null) navigate(`/dashboard/${accountId}${rest}`);
    },
    [location.pathname, navigate],
  );

  return <OrgCtx.Provider value={{ accounts, current, select, withScope, reload, loading }}>{children}</OrgCtx.Provider>;
}

export const useOrg = () => useContext(OrgCtx);
