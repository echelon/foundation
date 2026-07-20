import { Link } from "react-router-dom";
import { ArrowRight, Activity, DollarSign, TrendingUp, Users } from "lucide-react";
import { Badge, Button, Card } from "@/components/ui";
import { RequestRow } from "./RequestBits";
import { BALANCE, REQUESTS, USAGE_14D, END_USERS } from "@/lib/mock-api";
import { useOrg } from "@/lib/org";

export default function Overview() {
  const { withScope } = useOrg();
  const totalReq = USAGE_14D.reduce((s, d) => s + d.requests, 0);
  const maxSpend = Math.max(...USAGE_14D.map((d) => d.spend));
  const recent = REQUESTS.slice(0, 6);

  return (
    <div className="space-y-8">
      <div>
        <h1 className="font-display text-3xl font-bold tracking-tight">Overview</h1>
        <p className="mt-1 text-muted">Usage across everything that transited your account in the last 14 days.</p>
      </div>

      {/* ── Stat cards ── */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard icon={DollarSign} label="Credits remaining" value={`$${BALANCE.credits.toFixed(2)}`} sub="Auto-reload on" />
        <StatCard icon={TrendingUp} label="Spend (month to date)" value={`$${BALANCE.monthToDateSpend.toLocaleString()}`} sub="+18% vs. last month" />
        <StatCard icon={Activity} label="Requests (14d)" value={totalReq.toLocaleString()} sub="Across all end-users" />
        <StatCard icon={Users} label="Active end-users" value={String(END_USERS.length)} sub="Your customers' users" />
      </div>

      {/* ── Usage chart ── */}
      <Card className="p-6">
        <div className="mb-6 flex items-center justify-between">
          <div>
            <h2 className="font-display font-semibold">Daily spend</h2>
            <p className="text-sm text-muted">Last 14 days · USD</p>
          </div>
          <Badge tone="green">Live</Badge>
        </div>
        <div className="flex h-48 items-end gap-2">
          {USAGE_14D.map((d) => (
            <div key={d.day} className="group flex flex-1 flex-col items-center gap-2">
              <div className="relative flex w-full flex-1 items-end">
                <div
                  className="w-full rounded-t-md brand-gradient transition group-hover:opacity-80"
                  style={{ height: `${(d.spend / maxSpend) * 100}%` }}
                />
                <div className="pointer-events-none absolute -top-8 left-1/2 -translate-x-1/2 whitespace-nowrap rounded-md bg-fg px-2 py-1 text-xs font-medium text-bg opacity-0 transition group-hover:opacity-100">
                  ${d.spend.toFixed(0)}
                </div>
              </div>
              <span className="text-[10px] text-muted">{d.day.split(" ")[1]}</span>
            </div>
          ))}
        </div>
      </Card>

      {/* ── Recent requests ── */}
      <Card className="overflow-hidden">
        <div className="flex items-center justify-between border-b border-border px-5 py-4">
          <div>
            <h2 className="font-display font-semibold">Recent requests</h2>
            <p className="text-sm text-muted">Everything that transited the account.</p>
          </div>
          <Link to={withScope("/requests")}>
            <Button variant="ghost" size="sm">
              View all <ArrowRight className="h-4 w-4" />
            </Button>
          </Link>
        </div>
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
              {recent.map((r) => (
                <RequestRow key={r.id} r={r} />
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}

function StatCard({ icon: Icon, label, value, sub }: { icon: typeof DollarSign; label: string; value: string; sub: string }) {
  return (
    <Card className="p-5">
      <div className="flex items-center gap-2 text-sm text-muted">
        <Icon className="h-4 w-4" /> {label}
      </div>
      <div className="mt-2 font-display text-2xl font-bold">{value}</div>
      <div className="mt-1 text-xs text-muted">{sub}</div>
    </Card>
  );
}
