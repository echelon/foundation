import { Cpu, X } from "lucide-react";
import { Button, Card } from "@/components/ui";
import { type UsageReport } from "@/surfaces/comparison/api";
import { usageModelName } from "./usageData";
import type { UsageResult } from "./useUsageReport";

export function UsageModels({ result, report, model, onModel }: {
  result?: UsageResult; report?: UsageReport; model: string; onModel: (model: string) => void;
}) {
  const catalog = result?.report;
  const ranked = (catalog?.groups ?? []).flatMap(({ key, requests }) => key === null ? [] : [{ model: key, requests }])
    .sort((a, b) => b.requests - a.requests || a.model.localeCompare(b.model));
  const visible = ranked.slice(0, 5);
  const totalRequests = catalog?.totals.requests ?? 0;
  const name = usageModelName(model);
  const latency = report?.totals.avg_success_latency_ms;

  return (
    <Card className="min-w-0 p-4 sm:p-5">
      <h2 className="flex items-center gap-2 font-display text-lg font-semibold"><Cpu className="h-4 w-4 text-brand" aria-hidden /> {model ? "Selected model" : "Model activity"}</h2>
      {model ? (
        <div className="mt-4">
          <div className="flex items-start justify-between gap-2">
            <div className="min-w-0">
              <h3 className="break-all font-display text-xl font-semibold">{name}</h3>
              {name !== model && <p className="mt-1 break-all font-mono text-xs text-muted">{model}</p>}
            </div>
            <Button type="button" size="sm" variant="ghost" aria-label="Show all models" onClick={() => onModel("")} className="shrink-0 rounded-md px-2">
              <X className="h-4 w-4" aria-hidden />
            </Button>
          </div>
          {report && <dl className="mt-4 divide-y divide-border border-y border-border text-sm">
            <div className="flex flex-wrap items-center justify-between gap-x-2 py-3"><dt className="text-muted">Succeeded</dt><dd className="font-semibold tabular-nums">{report.totals.succeeded.toLocaleString()}</dd></div>
            <div className="flex flex-wrap items-center justify-between gap-x-2 py-3"><dt className="text-muted">Avg. success latency</dt><dd className="font-semibold tabular-nums">{latency == null ? "—" : `${latency.toLocaleString()} ms`}</dd></div>
          </dl>}
        </div>
      ) : (
        <>
          <p className="mt-1 text-sm text-muted">{catalog?.next_after ? "Most requests on the first model page" : "Most requests in this period"}</p>
          {visible.length > 0 ? (
            <ol className="mt-3 space-y-1">
              {visible.map((item) => {
                const fraction = totalRequests > 0 ? item.requests / totalRequests : 0;
                const label = usageModelName(item.model);
                return <li key={item.model}>
                  <button type="button" onClick={() => onModel(item.model)} title={item.model}
                    className="w-full rounded-lg border border-transparent px-2 py-2 text-left hover:border-border hover:bg-surface2 focus-visible:outline-brand">
                    <span className="flex items-baseline justify-between gap-2 text-sm">
                      <span className="min-w-0 truncate font-semibold" title={label}>{label}</span>
                      <span className="shrink-0 font-mono text-xs tabular-nums text-muted">{item.requests.toLocaleString()}</span>
                    </span>
                    <span aria-hidden className="mt-2 block h-1.5 overflow-hidden rounded-full bg-surface2">
                      <span className="block h-full rounded-full bg-brand" style={{ width: `${fraction * 100}%` }} />
                    </span>
                  </button>
                </li>;
              })}
            </ol>
          ) : <p className="mt-4 text-sm text-muted" role="status">{!result ? "Loading models…" : result.error ? "Model activity is unavailable." : "No model activity in this period."}</p>}
        </>
      )}
    </Card>
  );
}
