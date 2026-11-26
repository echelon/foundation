import { Activity, CircleDollarSign, Layers3, Timer } from "lucide-react";
import { Card } from "@/components/ui";
import { formatNanoUsd, formatProviderCharge, providerLabel, type ProviderChargeTotal, type UsageReport } from "@/surfaces/comparison/api";

export function UsageSummary({ report }: { report: UsageReport }) {
  const total = report.totals;
  const statuses = [
    [total.succeeded, "succeeded"], [total.failed, "failed"], [total.canceled, "canceled"],
    [total.in_progress, "in progress"], [total.awaiting_reconciliation, "unconfirmed"],
  ] as const;
  const activity = statuses.filter(([count]) => count > 0).map(([count, label]) => `${count.toLocaleString()} ${label}`).join(" · ");
  const mockOnly = report.source === "mock_provider" && total.requests > 0;
  const stats = [
    { icon: Activity, label: "Requests", value: total.requests.toLocaleString(), detail: activity || "No requests in this period" },
    { icon: Layers3, label: "Completed outputs", value: total.completed_outputs.toLocaleString(),
      detail: mockOnly && total.completed_outputs > 0
        ? `${formatNanoUsd(total.mock_reported_usd_nanos, total.completed_outputs)} fixture cost per output`
        : total.requests ? `${percent.format(total.succeeded / total.requests)} of requests succeeded` : "No outputs in this period" },
    { icon: CircleDollarSign, label: mockOnly ? "Fixture-reported USD" : "Provider-reported USD",
      value: formatNanoUsd(mockOnly ? total.mock_reported_usd_nanos : total.provider_reported_usd_nanos),
      detail: mockOnly ? "In-process fixture activity" : total.provider_reports_missing_cost > 0
        ? `${total.provider_reports_missing_cost.toLocaleString()} reports without a USD amount`
        : total.provider_input_tokens != null || total.provider_output_tokens != null
          ? `${total.provider_input_tokens?.toLocaleString() ?? "—"} input / ${total.provider_output_tokens?.toLocaleString() ?? "—"} output tokens`
          : total.provider_reported_usd_nanos === null ? "No USD amount reported" : "Recorded provider amounts" },
    { icon: Timer, label: "In-progress estimates", value: formatNanoUsd(total.in_progress_estimated_usd_nanos),
      detail: `${formatNanoUsd(total.estimated_usd_nanos)} quoted across all requests` },
  ];

  return (
    <dl className="grid min-w-0 grid-cols-2 gap-2 sm:gap-3 xl:grid-cols-4">
      {stats.map(({ icon: Icon, label, value, detail }) => (
        <Card className="min-w-0 p-3 sm:p-4" key={label}>
          <dt className="flex items-center gap-1.5 text-xs font-medium text-muted sm:gap-2 sm:text-sm">
            <span className="grid h-6 w-6 shrink-0 place-items-center rounded-md bg-brand/10 text-brand sm:h-7 sm:w-7"><Icon className="h-3.5 w-3.5 sm:h-4 sm:w-4" aria-hidden /></span>
            {label}
          </dt>
          <dd className="mt-2 break-words font-display text-base font-bold tabular-nums tracking-tight sm:text-2xl">{value}</dd>
          <dd className="mt-1 break-words text-xs leading-4 text-muted sm:leading-5">{detail}</dd>
        </Card>
      ))}
    </dl>
  );
}

const percent = new Intl.NumberFormat(undefined, { style: "percent", maximumFractionDigits: 1 });

export function UsageCharges({ report }: { report: UsageReport }) {
  const { totals, source } = report;
  const charges = totals.provider_charges ?? [];
  const settled = totals.supplier_settled_requests ?? 0;
  const fixtures = source === "mixed";
  if (!charges.length && !settled && !fixtures) return null;
  const title = charges.length ? settled ? "Provider charges and settlements" : "Provider charges"
    : settled ? "Supplier-settled costs" : "Fixture costs";

  return (
    <Card className="p-4 sm:p-5">
      <h2 className="font-display text-lg font-semibold">{title}</h2>
      <dl className="mt-4 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {charges.map((charge) => <ProviderChargeItem key={`${charge.provider}:${charge.unit}`} charge={charge} />)}
        {settled > 0 && <ChargeItem label="Supplier-settled USD" value={formatNanoUsd(totals.supplier_settled_usd_nanos ?? null)}
          detail={`${settled.toLocaleString()} settled ${settled === 1 ? "request" : "requests"}`} />}
        {fixtures && <ChargeItem label="Fixture-reported USD" value={formatNanoUsd(totals.mock_reported_usd_nanos)}
          detail="In-process fixture activity" />}
      </dl>
    </Card>
  );
}

function ProviderChargeItem({ charge }: { charge: ProviderChargeTotal }) {
  const count = charge.observations;
  return <ChargeItem label={providerLabel(charge.provider)} value={formatProviderCharge(charge)} detail={`${count.toLocaleString()} ${count === 1 ? "report" : "reports"}`} />;
}

function ChargeItem({ label, value, detail }: { label: string; value: string; detail: string }) {
  return <div className="min-w-0 border-l border-border pl-3">
    <dt className="break-words text-sm font-medium text-muted">{label}</dt>
    <dd className="mt-1 break-words font-mono text-lg font-semibold">{value}</dd>
    <dd className="mt-1 text-xs text-muted">{detail}</dd>
  </div>;
}
