import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { api, type ApiUser } from "./api";

/**
 * Real auth state, backed by the api-gateway session cookie. Wraps the whole app;
 * `useAuth()` anywhere gives the current user (or null) plus login/signup/logout.
 *
 * `loading` is true only during the initial `/me` probe on page load, so route
 * guards can wait instead of flashing a redirect.
 */
type AuthContextValue = {
  user: ApiUser | null;
  /** Connected SSO providers ("google", …) from /me — login-method factoids in settings. */
  ssoProviders: string[];
  loading: boolean;
  login: (email: string, password: string) => Promise<ApiUser>;
  /** `invitation` (code or token) redeems an invite during signup — see api.signup. */
  signup: (email: string, password: string, displayName?: string, invitation?: string) => Promise<ApiUser>;
  logout: () => Promise<void>;
  /** Re-fetch `/me` (e.g. after changing email so `email_verified` refreshes). */
  refresh: () => Promise<void>;
};

const AuthCtx = createContext<AuthContextValue>({
  user: null,
  ssoProviders: [],
  loading: true,
  login: async () => {
    throw new Error("AuthProvider missing");
  },
  signup: async () => {
    throw new Error("AuthProvider missing");
  },
  logout: async () => {},
  refresh: async () => {},
});

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<ApiUser | null>(null);
  const [ssoProviders, setSsoProviders] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    api
      .me()
      .then((r) => {
        if (!cancelled) {
          setUser(r.user);
          setSsoProviders(r.sso_providers);
        }
      })
      .catch(() => {
        if (!cancelled) setUser(null); // no/expired session — that's fine
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // login/signup responses don't carry the avatar — backfill from /me in the background.
  const backfill = () =>
    void api
      .me()
      .then((m) => {
        setUser(m.user);
        setSsoProviders(m.sso_providers);
      })
      .catch(() => {});

  const login = useCallback(async (email: string, password: string) => {
    const r = await api.login(email, password);
    setUser(r.user);
    backfill();
    return r.user;
  }, []);

  const signup = useCallback(async (email: string, password: string, displayName?: string, invitation?: string) => {
    const r = await api.signup(email, password, displayName, invitation);
    setUser(r.user);
    backfill();
    return r.user;
  }, []);

  const logout = useCallback(async () => {
    try {
      await api.logout();
    } finally {
      setUser(null); // clear locally even if the request failed
    }
  }, []);

  const refresh = useCallback(async () => {
    try {
      const r = await api.me();
      setUser(r.user);
      setSsoProviders(r.sso_providers);
    } catch {
      setUser(null);
      setSsoProviders([]);
    }
  }, []);

  return <AuthCtx.Provider value={{ user, ssoProviders, loading, login, signup, logout, refresh }}>{children}</AuthCtx.Provider>;
}

export const useAuth = () => useContext(AuthCtx);
