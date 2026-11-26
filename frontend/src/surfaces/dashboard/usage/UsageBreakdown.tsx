import { Link } from "react-router-dom";
import { ChevronLeft, ChevronRight, Table2 } from "lucide-react";
import { Button, Card, Select } from "@/components/ui";
import { formatNanoUsd, formatProviderCharge, providerLabel, type UsageGroupBy, type UsageReport } from "@/surfaces/comparison/api";
import { groupLabels, shortDay, usageModelName } from "./usageData";
import type { UsageResult } from "./useUsageReport";

export function UsageBreakdown({ result, groupBy, page, onGroup, onPrevious, onNext, onRetry, requestLink }: {
  result?: UsageResult; groupBy: UsageGroupBy; page: number;
  onGroup: (group: UsageGroupBy) => void; onPrevious: () => void; onNext: (cursor: string) => void; onRetry: () => void;
  requestLink?: (report: UsageReport, group: UsageReport["groups"][number]) => string;
}) {
  const report = result?.report;
  return (
    <Card className="min-w-0 overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-3 px-4 py-4 sm:px-5">
        <h2 className="flex items-center gap-2 font-display text-lg font-semibold"><Table2 className="h-4 w-4 text-brand" aria-hidden /> Usage breakdown</h2>
        <label className="flex items-center gap-2 text-sm text-muted">Group by
          <Select className="h-9 rounded-lg" value={groupBy} onChange={(event) => onGroup(event.target.value as UsageGroupBy)}>
            {Object.entries(groupLabels).map(([key, label]) => <option key={key} value={key}>{label}</option>)}
          </Select>
        </label>
      </div>
      {!result && <p className="border-t border-border px-5 py-6 text-sm text-muted" role="status">Loading breakdown…</p>}
      {result?.error && <div className="flex flex-wrap items-center gap-3 border-t border-border px-5 py-4" role="alert">
        <p className="text-sm text-rose-700 dark:text-rose-300">{result.error}</p>
        <Button size="sm" variant="outline" className="rounded-md" onClick={onRetry}>Retry breakdown</Button>
      </div>}
      {report && (report.groups.length > 0 ? <GroupTable report={report} requestLink={requestLink} /> : (
        <p className="border-t border-border px-5 py-6 text-sm text-muted">No groups to display.</p>
      ))}
      {(page > 1 || report?.next_after) && <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border px-4 py-3 sm:px-5">
        <span className="text-sm text-muted">Page {page}</span>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" variant="outline" className="rounded-md" disabled={page === 1} onClick={onPrevious}>
            <ChevronLeft className="h-4 w-4" aria-hidden /> Previous</Button>
          <Button size="sm" variant="outline" className="rounded-md" disabled={!report?.next_after} onClick={() => {
            if (report?.next_after) onNext(report.next_after);
          }}>Next <ChevronRight className="h-4 w-4" aria-hidden /></Button>
        </div>
      </div>}
    </Card>
  );
}

const numberCell = "whitespace-nowrap px-4 py-3 text-right font-mono tabular-nums";

function GroupTable({ report, requestLink }: {
  report: UsageReport; requestLink?: (report: UsageReport, group: UsageReport["groups"][number]) => string;
}) {
  const { totals, group_by: groupBy } = report;
  const label = groupLabels[groupBy];
  const fixture = report.source !== "provider";
  const provider = report.source !== "mock_provider";
  const charges = Boolean(totals.provider_charges?.length);
  const settlements = (totals.supplier_settled_requests ?? 0) > 0;
  const columns = ["Requests", "Outputs", ...(provider ? ["Provider USD"] : []), ...(fixture ? ["Fixture USD"] : []),
    ...(charges ? ["Provider charges"] : []), ...(settlements ? ["Settled USD"] : []), "In-progress estimate"];

  return (
    <div className="overflow-x-auto border-t border-border focus-visible:outline-brand" role="region" aria-label={`Usage by ${label.toLowerCase()}`} tabIndex={0}>
      <table className="w-full text-left text-sm">
        <thead className="border-b border-border bg-surface2/70 text-xs text-muted">
          <tr>
            <th scope="col" className="sticky left-0 z-10 whitespace-nowrap bg-surface2 px-4 py-3 font-semibold sm:pl-5">{label}</th>
            {columns.map((column) => <th key={column} scope="col" className="whitespace-nowrap px-4 py-3 text-right font-semibold">{column}</th>)}
          </tr>
        </thead>
        <tbody className="divide-y divide-border">
          {report.groups.map((group) => {
            const name = groupLabel(groupBy, group.key);
            const groupCharges = group.provider_charges ?? [];
            return <tr key={group.key === null ? "unattributed" : `group:${group.key}`} className="hover:bg-surface2/30">
              <th scope="row" className="sticky left-0 z-10 min-w-40 max-w-56 break-words bg-surface px-4 py-3 font-medium sm:pl-5">
                {requestLink ? <Link className="underline decoration-border underline-offset-4 hover:decoration-brand focus-visible:outline-brand"
                  to={requestLink(report, group)} aria-label={`View requests for ${name}`}>{name}</Link> : name}
                {groupBy === "model" && group.key !== name && <span className="mt-0.5 block font-mono text-xs text-muted">{group.key}</span>}
              </th>
              <td className={numberCell}>{group.requests.toLocaleString()}</td>
              <td className={numberCell}>{group.completed_outputs.toLocaleString()}</td>
              {provider && <td className={numberCell}>{formatNanoUsd(group.provider_reported_usd_nanos)}</td>}
              {fixture && <td className={numberCell}>{formatNanoUsd(group.mock_reported_usd_nanos)}</td>}
              {charges && <td className={numberCell}>
                {groupCharges.length === 0 ? "—" : groupCharges.map((charge) => <div key={`${charge.provider}:${charge.unit}`}>
                  {groupBy !== "provider" && <span className="block text-xs text-muted">{providerLabel(charge.provider)}</span>}
                  {formatProviderCharge(charge)}
                </div>)}
              </td>}
              {settlements && <td className={numberCell}>{formatNanoUsd(group.supplier_settled_usd_nanos ?? null)}</td>}
              <td className={numberCell}>{formatNanoUsd(group.in_progress_estimated_usd_nanos)}</td>
            </tr>;
          })}
        </tbody>
      </table>
    </div>
  );
}

function groupLabel(groupBy: UsageGroupBy, key: string | null) {
  if (key === null) return "Unattributed";
  if (groupBy === "model") return usageModelName(key);
  if (groupBy === "provider") return providerLabel(key);
  if (groupBy === "day") return shortDay(key);
  return key;
}
