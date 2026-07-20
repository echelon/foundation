import { useCallback, useEffect, useState } from "react";
import { Badge, Button, Card, Select } from "@/components/ui";
import { api, ApiError, type ApiMember, type AuditItem, type AuditPage } from "@/lib/api";
import { useOrg } from "@/lib/org";

type Tab = "mine" | "org" | "members";

/**
 * Audit log page, tabbed: My Activity | Org Activity | Org Member Activity. Everyone gets the
 * first tab (private rows included); the org tabs appear only for owners/admins of the
 * currently-selected organization. All lists are keyset-paginated ("Load more").
 */
export default function AuditLog() {
  const { current, loading } = useOrg();
  const [myRole, setMyRole] = useState<string | null>(null);
  const [members, setMembers] = useState<ApiMember[]>([]);
  const [tab, setTab] = useState<Tab>("mine");

  useEffect(() => {
    setMyRole(null);
    setTab("mine"); // switching orgs may change what's visible — land somewhere always-valid
    if (!current) return;
    let cancelled = false;
    api
      .listMembers(current.account_id)
      .then((r) => {
        if (!cancelled) {
          setMyRole(r.your_role);
          setMembers(r.members);
        }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [current]);

  if (loading) return null;
  const isOrgAdmin = myRole === "owner" || myRole === "admin";

  const tabs: { key: Tab; label: string }[] = [
    { key: "mine", label: "My Activity" },
    ...(isOrgAdmin && current
      ? [
          { key: "org" as Tab, label: "Org Activity" },
          { key: "members" as Tab, label: "Org Member Activity" },
        ]
      : []),
  ];

  return (
    <div className="max-w-3xl space-y-6">
      <div>
        <h1 className="font-display text-3xl font-bold tracking-tight">Audit log</h1>
        <p className="mt-1 text-muted">
          Your account activity{isOrgAdmin && current ? ` — and ${current.name ?? "your organization"}'s` : ""}.
        </p>
      </div>

      {tabs.length > 1 && (
        <div className="inline-flex rounded-lg border border-border bg-surface p-1">
          {tabs.map((t) => (
            <button
              key={t.key}
              onClick={() => setTab(t.key)}
              className={
                tab === t.key
                  ? "rounded-md px-3 py-1.5 text-xs font-medium brand-gradient text-white shadow-glow"
                  : "rounded-md px-3 py-1.5 text-xs font-medium text-muted transition hover:text-fg"
              }
            >
              {t.label}
            </button>
          ))}
        </div>
      )}

      {tab === "mine" && (
        <AuditSection
          title="My activity"
          subtitle="Everything on your user — including items only you can see."
          fetchPage={(before) => api.userAuditLogs(before)}
        />
      )}
      {tab === "org" && isOrgAdmin && current && (
        <AuditSection
          key={`org-${current.account_id}`}
          title="Organization activity"
          subtitle="Membership, invitations, keys, and billing-profile events (admins only)."
          fetchPage={(before) => api.accountAuditLogs(current.account_id, before)}
        />
      )}
      {tab === "members" && isOrgAdmin && current && (
        <MemberAuditSection accountId={current.account_id} members={members} />
      )}
    </div>
  );
}

/* ── A paginated audit list ── */

function AuditSection({
  title,
  subtitle,
  fetchPage,
}: {
  title: string;
  subtitle: string;
  fetchPage: (before?: string) => Promise<AuditPage>;
}) {
  const [items, setItems] = useState<AuditItem[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [state, setState] = useState<"loading" | "idle" | "error">("loading");

  const loadFirst = useCallback(async () => {
    setState("loading");
    try {
      const page = await fetchPage(undefined);
      setItems(page.items);
      setCursor(page.next_before);
      setState("idle");
    } catch {
      setState("error");
    }
  }, [fetchPage]);

  useEffect(() => {
    void loadFirst();
  }, [loadFirst]);

  const loadMore = async () => {
    if (!cursor) return;
    setState("loading");
    try {
      const page = await fetchPage(cursor);
      setItems((prev) => [...prev, ...page.items]);
      setCursor(page.next_before);
      setState("idle");
    } catch {
      setState("error");
    }
  };

  return (
    <Card className="overflow-hidden">
      <div className="border-b border-border px-5 py-4">
        <h2 className="font-display font-semibold">{title}</h2>
        <p className="text-xs text-muted">{subtitle}</p>
      </div>
      {state === "error" ? (
        <p className="px-5 py-4 text-sm text-rose-500">Couldn't load this log.</p>
      ) : items.length === 0 && state === "idle" ? (
        <p className="px-5 py-4 text-sm text-muted">Nothing here yet.</p>
      ) : (
        <ul>
          {items.map((item) => (
            <AuditRow key={item.id} item={item} />
          ))}
        </ul>
      )}
      {(cursor || state === "loading") && (
        <div className="border-t border-border px-5 py-3">
          <Button variant="outline" size="sm" onClick={loadMore} disabled={state === "loading" || !cursor}>
            {state === "loading" ? "Loading…" : "Load more"}
          </Button>
        </div>
      )}
    </Card>
  );
}

/* ── Member picker + their org-visible history ── */

function MemberAuditSection({ accountId, members }: { accountId: string; members: ApiMember[] }) {
  const [userId, setUserId] = useState<string>("");

  useEffect(() => {
    setUserId("");
  }, [accountId]);

  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border px-5 py-4">
        <div>
          <h2 className="font-display font-semibold">Member activity</h2>
          <p className="text-xs text-muted">A member's history — organization-visible items only.</p>
        </div>
        <Select value={userId} onChange={(e) => setUserId(e.target.value)} className="w-56">
          <option value="">Choose a member…</option>
          {members.map((m) => (
            <option key={m.user_id} value={m.user_id}>
              {m.display_name ?? m.email}
            </option>
          ))}
        </Select>
      </div>
      {userId ? (
        <InnerMemberList key={userId} accountId={accountId} userId={userId} />
      ) : (
        <p className="px-5 py-4 text-sm text-muted">Pick a member to see their organization-visible activity.</p>
      )}
    </Card>
  );
}

function InnerMemberList({ accountId, userId }: { accountId: string; userId: string }) {
  const [page, setPage] = useState<AuditPage | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .memberAuditLogs(accountId, userId)
      .then((p) => {
        if (!cancelled) setPage(p);
      })
      .catch((err) => {
        if (!cancelled) setError(err instanceof ApiError ? err.message : "Couldn't load.");
      });
    return () => {
      cancelled = true;
    };
  }, [accountId, userId]);

  const loadMore = async () => {
    if (!page?.next_before) return;
    const next = await api.memberAuditLogs(accountId, userId, page.next_before);
    setPage({ items: [...page.items, ...next.items], next_before: next.next_before });
  };

  if (error) return <p className="px-5 py-4 text-sm text-rose-500">{error}</p>;
  if (!page) return <p className="px-5 py-4 text-sm text-muted">Loading…</p>;
  if (page.items.length === 0) return <p className="px-5 py-4 text-sm text-muted">No organization-visible activity.</p>;
  return (
    <>
      <ul>
        {page.items.map((item) => (
          <AuditRow key={item.id} item={item} />
        ))}
      </ul>
      {page.next_before && (
        <div className="border-t border-border px-5 py-3">
          <Button variant="outline" size="sm" onClick={() => void loadMore()}>
            Load more
          </Button>
        </div>
      )}
    </>
  );
}

/* ── One row ── */

function AuditRow({ item }: { item: AuditItem }) {
  return (
    <li className="flex items-start justify-between gap-3 border-b border-border/60 px-5 py-3 last:border-0">
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-sm font-medium text-fg">{humanizeEvent(item.event)}</span>
          {item.is_org_visible === false && <Badge tone="muted">only you</Badge>}
        </div>
        {summarize(item) && <div className="mt-0.5 truncate text-xs text-muted">{summarize(item)}</div>}
      </div>
      <div className="shrink-0 text-right text-xs text-muted">
        <div>{new Date(item.created_at).toLocaleString()}</div>
        {item.ip && <div className="font-mono">{item.ip}</div>}
      </div>
    </li>
  );
}

function humanizeEvent(event: string): string {
  return event.replaceAll("_", " ");
}

/** A terse one-line summary from the event's data payload. */
function summarize(item: AuditItem): string | null {
  const d = item.data;
  if (!d) return null;
  if (typeof d.from !== "undefined" || typeof d.to !== "undefined") {
    return `${d.from ?? "—"} → ${d.to ?? "—"}`;
  }
  if (typeof d.old_email === "string" && typeof d.new_email === "string") {
    return `${d.old_email} → ${d.new_email}`;
  }
  const parts = Object.entries(d)
    .filter(([, v]) => typeof v === "string" || typeof v === "number")
    .map(([k, v]) => `${k}: ${v}`);
  return parts.length > 0 ? parts.join(" · ") : null;
}
