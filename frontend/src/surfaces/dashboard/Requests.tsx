import { useMemo, useState } from "react";
import { Info, Search } from "lucide-react";
import { Badge, Card, Input } from "@/components/ui";
import { RequestRow } from "./RequestBits";
import { REQUESTS, END_USERS, type JobStatus } from "@/lib/mock-api";
import { cn } from "@/lib/cn";

type Scope = "mine" | "account";

export default function Requests() {
  const [scope, setScope] = useState<Scope>("account");
  const [endUser, setEndUser] = useState<string>("all");
  const [status, setStatus] = useState<JobStatus | "all">("all");
  const [query, setQuery] = useState("");

  const rows = useMemo(() => {
    const q = query.trim().toLowerCase();
    return REQUESTS.filter((r) => {
      if (scope === "mine" && !r.mine) return false;
      if (endUser !== "all" && r.endUser !== endUser) return false;
      if (status !== "all" && r.status !== status) return false;
      if (q && !(`${r.id} ${r.modelSlug} ${r.summary} ${r.endUser ?? ""}`.toLowerCase().includes(q))) return false;
      return true;
    });
  }, [scope, endUser, status, query]);

  const totalCost = rows.reduce((s, r) => s + r.costUsd, 0);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-3xl font-bold tracking-tight">Requests</h1>
        <p className="mt-1 text-muted">Inspect every generation on the account — yours and your customers' end-users.</p>
      </div>

      {/* ── Scope explainer ── */}
      <Card className="flex items-start gap-3 border-brand/30 bg-brand/[0.05] p-4">
        <Info className="mt-0.5 h-4 w-4 shrink-0 text-brand" />
        <p className="text-sm text-muted">
          <strong className="text-fg">Account scope</strong> shows everything that transited your account,
          including traffic your customers' end-users generated through your integration (tagged with{" "}
          <code className="rounded bg-surface2 px-1.5 py-0.5">end_user</code>). Switch to{" "}
          <strong className="text-fg">My requests</strong> to see only what you ran in the dashboard.
        </p>
      </Card>

      {/* ── Controls ── */}
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div className="inline-flex rounded-lg border border-border bg-surface p-1">
          {(["account", "mine"] as Scope[]).map((s) => (
            <button
              key={s}
              onClick={() => setScope(s)}
              className={cn(
                "rounded-md px-4 py-1.5 text-sm font-medium transition",
                scope === s ? "brand-gradient text-white shadow-glow" : "text-muted hover:text-fg",
              )}
            >
              {s === "account" ? "All account traffic" : "My requests"}
            </button>
          ))}
        </div>

        <div className="flex flex-wrap gap-2">
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted" />
            <Input placeholder="Search requests…" value={query} onChange={(e) => setQuery(e.target.value)} className="w-48 pl-9" />
          </div>
          <select
            value={endUser}
            onChange={(e) => setEndUser(e.target.value)}
            disabled={scope === "mine"}
            className="h-10 rounded-lg border border-border bg-bg px-3 text-sm text-fg outline-none focus:border-brand focus:ring-2 focus:ring-brand/25 disabled:opacity-50"
          >
            <option value="all">All end-users</option>
            {END_USERS.map((u) => (
              <option key={u} value={u}>
                {u}
              </option>
            ))}
          </select>
          <select
            value={status}
            onChange={(e) => setStatus(e.target.value as JobStatus | "all")}
            className="h-10 rounded-lg border border-border bg-bg px-3 text-sm text-fg outline-none focus:border-brand focus:ring-2 focus:ring-brand/25"
          >
            <option value="all">Any status</option>
            <option value="succeeded">Succeeded</option>
            <option value="running">Running</option>
            <option value="failed">Failed</option>
          </select>
        </div>
      </div>

      {/* ── Summary ── */}
      <div className="flex flex-wrap items-center gap-3 text-sm text-muted">
        <Badge>{rows.length} requests</Badge>
        <span>·</span>
        <span>
          Total cost <strong className="text-fg">${totalCost.toFixed(2)}</strong>
        </span>
      </div>

      {/* ── Table ── */}
      <Card className="overflow-hidden">
        <div className="scrollbar-thin overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="border-b border-border text-left text-muted">
              <tr>
                <th className="px-5 py-3 font-medium">Request</th>
                <th className="px-5 py-3 font-medium">Model</th>
                <th className="px-5 py-3 font-medium">Source</th>
                <th className="px-5 py-3 font-medium">End-user</th>
                <th className="px-5 py-3 font-medium">Status</th>
                <th className="px-5 py-3 text-right font-medium">Cost</th>
              </tr>
            </thead>
            <tbody>
              {rows.length ? (
                rows.map((r) => <RequestRow key={r.id} r={r} />)
              ) : (
                <tr>
                  <td colSpan={6} className="px-5 py-16 text-center text-muted">
                    No requests match these filters.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}
