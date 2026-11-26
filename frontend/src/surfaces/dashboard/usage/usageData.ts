import { formatNanoUsd, type UsageFilters, type UsageGroupBy, type UsageMetrics, type UsageReport } from "@/surfaces/comparison/api";
import { dateRangeError, shiftDate } from "@/surfaces/requests/historyDiagnostics";
import { catalogModelName } from "@/surfaces/requests/modelOptions";

export type UsageSelection = Omit<UsageFilters, "group_by">;
export type UsageWindow = UsageReport["window"];

export const groupLabels: Record<UsageGroupBy, string> = {
  model: "Model", provider: "Selected provider", client: "Client reference", app: "App reference", day: "UTC day",
};

export const periods = [7, 30, 90] as const;

export function recentWindow(days: number): UsageWindow {
  const today = new Date().toISOString().slice(0, 10);
  return { from: shiftDate(today, 1 - days), to: shiftDate(today, 1) };
}

export function customWindow(from: string, through: string) {
  const window = { from, to: shiftDate(through, 1) };
  const error = dateRangeError(window);
  return { window, error };
}

const dayFormat = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", timeZone: "UTC" });
const dateFormat = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric", timeZone: "UTC" });

export function shortDay(date: string) {
  return dayFormat.format(new Date(date));
}

export function windowLabel({ from, to }: UsageWindow) {
  return dateFormat.formatRange(new Date(from), new Date(shiftDate(to, -1)));
}

export function usageModelName(model: string) {
  return catalogModelName(model) ?? model;
}

type Metric = {
  label: string;
  format: (value: bigint) => string;
  read: (totals: UsageMetrics) => bigint | null;
  visible?: (report: UsageReport) => boolean;
};

const count = (value: bigint) => value.toLocaleString();
const usd = (value: bigint) => formatNanoUsd(value.toString());

export const metrics = {
  requests: { label: "Requests", format: count, read: (totals) => BigInt(totals.requests) },
  outputs: { label: "Completed outputs", format: count, read: (totals) => BigInt(totals.completed_outputs) },
  provider: { label: "Provider-reported USD", format: usd,
    read: (totals) => totals.provider_reported_usd_nanos === null ? null : BigInt(totals.provider_reported_usd_nanos),
    visible: (report) => report.source !== "mock_provider" },
  fixture: { label: "Fixture-reported USD", format: usd, read: (totals) => BigInt(totals.mock_reported_usd_nanos),
    visible: (report) => report.source !== "provider" && report.totals.requests > 0 },
  estimates: { label: "In-progress estimates", format: usd, read: (totals) => BigInt(totals.in_progress_estimated_usd_nanos) },
  settlements: { label: "Supplier-settled USD", format: usd,
    read: (totals) => totals.supplier_settled_usd_nanos == null ? null : BigInt(totals.supplier_settled_usd_nanos),
    visible: (report) => (report.totals.supplier_settled_requests ?? 0) > 0 },
} satisfies Record<string, Metric>;

export type MetricId = keyof typeof metrics;

export function availableMetrics(report: UsageReport) {
  return (Object.keys(metrics) as MetricId[]).filter((key) => {
    const metric: Metric = metrics[key];
    return metric.visible?.(report) ?? true;
  });
}

export type UsageBucket = UsageWindow & { value: bigint | null; requests: number };

export function usageBuckets(
  window: UsageWindow, groups: UsageReport["groups"], metric: MetricId, interval: "day" | "week",
) {
  const byDay = new Map(groups.map((row) => [row.key, row]));
  const buckets: UsageBucket[] = [];
  for (let from = window.from; from < window.to; ) {
    const weekday = new Date(from).getUTCDay();
    const next = shiftDate(from, interval === "day" ? 1 : 7 - (weekday + 6) % 7);
    const to = next < window.to ? next : window.to;
    let requests = 0;
    let value: bigint | null = null;
    for (let day = from; day < to; day = shiftDate(day, 1)) {
      const row = byDay.get(day);
      if (!row) continue;
      requests += row.requests;
      const amount = metrics[metric].read(row);
      if (amount !== null) value = (value ?? 0n) + amount;
    }
    if (value === null && metric !== "provider" && metric !== "settlements") value = 0n;
    buckets.push({ from, to, value, requests });
    from = to;
  }
  return buckets;
}
