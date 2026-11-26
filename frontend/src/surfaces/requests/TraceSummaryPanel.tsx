import { Button } from "@/components/ui";
import type { EvidenceStep, TraceSummary } from "@/surfaces/comparison/api";

export function TraceSummaryPanel({ summary, steps, change, onSelect }: {
  summary: TraceSummary;
  steps: EvidenceStep[];
  change?: { message: string; stepId: string };
  onSelect: (stepId: string) => void;
}) {
  const available = (id: string) => steps.some((step) => step.id === id);
  const next = summary.next_check;
  return (
    <section aria-label="Request summary" className="space-y-3 border-b border-border px-4 py-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <h4 className="font-semibold">{summary.headline}</h4>
          {summary.detail && <p className="mt-1 text-xs text-muted">{summary.detail}</p>}
        </div>
        {available(next.step_id) && (
          <Button type="button" variant="outline" size="sm" onClick={() => onSelect(next.step_id)}>
            {next.label}
          </Button>
        )}
      </div>
      <dl className="flex flex-wrap gap-x-5 gap-y-2 text-xs">
        {summary.signals.map(({ label, value, tone, step_id }) => (
          <div key={label} className="min-w-0">
            <dt className="text-muted">{label}</dt>
            <dd className="mt-0.5 tc-debug-tone font-medium" data-tone={tone}>
              {available(step_id)
                ? <button type="button" className="rounded-sm text-left underline decoration-dotted underline-offset-2"
                    onClick={() => onSelect(step_id)}>{value}</button>
                : value}
            </dd>
          </div>
        ))}
      </dl>
      {change && (
        <div className="flex flex-wrap items-baseline gap-x-2 text-xs">
          <p role="status" className="min-w-0 break-words text-muted">
            <strong className="text-fg">What changed</strong> · {change.message}
          </p>
          {available(change.stepId) && (
            <button type="button" className="text-brand underline underline-offset-2"
              onClick={() => onSelect(change.stepId)}>Inspect change</button>
          )}
        </div>
      )}
    </section>
  );
}
