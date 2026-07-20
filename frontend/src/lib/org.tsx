import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { api, type ApiAccount } from "./api";
import { useAuth } from "./auth";

/**
 * Organization scope for the dashboard. Loads the user's memberships and tracks which org the
 * console is currently scoped to (persisted per user in localStorage). Members of multiple
 * organizations switch via the header switcher; every org-scoped page reads `current`.
 */
type OrgContextValue = {
  accounts: ApiAccount[];
  /** The currently-selected organization (null while loading / if none). */
  current: ApiAccount | null;
  select: (accountId: string) => void;
  reload: () => Promise<void>;
  loading: boolean;
};

const OrgCtx = createContext<OrgContextValue>({
  accounts: [],
  current: null,
  select: () => {},
  reload: async () => {},
  loading: true,
});

export function OrgProvider({ children }: { children: ReactNode }) {
  const { user } = useAuth();
  const [accounts, setAccounts] = useState<ApiAccount[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const storageKey = user ? `org:selected:${user.id}` : null;

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

  // Resolve the selection: stored choice if still a membership, else the oldest membership.
  const current = useMemo(() => {
    if (accounts.length === 0) return null;
    const stored = storageKey ? localStorage.getItem(storageKey) : null;
    const want = selectedId ?? stored;
    return accounts.find((a) => a.account_id === want) ?? accounts[0];
  }, [accounts, selectedId, storageKey]);

  const select = useCallback(
    (accountId: string) => {
      setSelectedId(accountId);
      if (storageKey) localStorage.setItem(storageKey, accountId);
    },
    [storageKey],
  );

  return <OrgCtx.Provider value={{ accounts, current, select, reload, loading }}>{children}</OrgCtx.Provider>;
}

export const useOrg = () => useContext(OrgCtx);
