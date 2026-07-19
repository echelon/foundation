import { CreditCard, Download } from "lucide-react";
import { Badge, Button, Card } from "@/components/ui";
import { BALANCE, INVOICES } from "@/lib/mock-api";

export default function Billing() {
  return (
    <div className="space-y-8">
      <div>
        <h1 className="font-display text-3xl font-bold tracking-tight">Billing</h1>
        <p className="mt-1 text-muted">Manage credits, payment method, and invoices.</p>
      </div>

      {/* ── Balance + auto-reload ── */}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="p-6">
          <div className="text-sm text-muted">Prepaid credits</div>
          <div className="mt-1 font-display text-4xl font-bold">${BALANCE.credits.toFixed(2)}</div>
          <div className="mt-2 text-sm text-muted">Month-to-date spend: ${BALANCE.monthToDateSpend.toLocaleString()}</div>
          <div className="mt-5 flex gap-3">
            <Button>Add credits</Button>
            <Button variant="outline">Download statement</Button>
          </div>
        </Card>

        <Card className="p-6">
          <div className="flex items-center justify-between">
            <div className="text-sm font-medium text-fg">Auto-reload</div>
            <Badge tone={BALANCE.autoReload ? "green" : "muted"}>{BALANCE.autoReload ? "On" : "Off"}</Badge>
          </div>
          <p className="mt-2 text-sm text-muted">
            When your balance falls below <strong className="text-fg">${BALANCE.autoReloadThreshold}</strong>, we
            automatically add <strong className="text-fg">${BALANCE.autoReloadAmount}</strong> so batches never
            stall.
          </p>
          <div className="mt-5 flex items-center gap-3 rounded-lg border border-border bg-surface2/50 p-3">
            <CreditCard className="h-5 w-5 text-muted" />
            <div className="text-sm">
              <div className="font-medium text-fg">Visa ending 4242</div>
              <div className="text-xs text-muted">Expires 08/28</div>
            </div>
            <Button variant="ghost" size="sm" className="ml-auto">
              Update
            </Button>
          </div>
        </Card>
      </div>

      {/* ── Tax note ── */}
      <Card className="p-5">
        <h2 className="font-display font-semibold">Tax & receipts</h2>
        <p className="mt-2 text-sm text-muted">
          Applicable sales tax is calculated and added to every top-up and invoice. Invoices are numbered
          sequentially without gaps for your records. Balances are non-refundable; contact support for
          exceptions.
        </p>
      </Card>

      {/* ── Invoices ── */}
      <Card className="overflow-hidden">
        <div className="border-b border-border px-5 py-4">
          <h2 className="font-display font-semibold">Invoices</h2>
        </div>
        <div className="scrollbar-thin overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="border-b border-border text-left text-muted">
              <tr>
                <th className="px-5 py-3 font-medium">Number</th>
                <th className="px-5 py-3 font-medium">Period</th>
                <th className="px-5 py-3 font-medium">Issued</th>
                <th className="px-5 py-3 font-medium">Status</th>
                <th className="px-5 py-3 text-right font-medium">Amount</th>
                <th className="px-5 py-3"></th>
              </tr>
            </thead>
            <tbody>
              {INVOICES.map((inv) => (
                <tr key={inv.number} className="border-b border-border/60 last:border-0">
                  <td className="px-5 py-3 font-mono text-[13px] text-fg">{inv.number}</td>
                  <td className="px-5 py-3 text-muted">{inv.period}</td>
                  <td className="px-5 py-3 text-muted">{inv.issued}</td>
                  <td className="px-5 py-3">
                    <Badge tone={inv.status === "paid" ? "green" : "amber"}>{inv.status}</Badge>
                  </td>
                  <td className="px-5 py-3 text-right font-medium text-fg">${inv.amount.toLocaleString()}</td>
                  <td className="px-5 py-3 text-right">
                    <button className="inline-flex items-center gap-1 text-xs text-muted hover:text-fg">
                      <Download className="h-3.5 w-3.5" /> PDF
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}
