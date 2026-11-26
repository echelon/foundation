import { useEffect, useState } from "react";
import type { ProjectScope } from "@/lib/gateway";
import { comparisonApi, errorMessage, type UsageFilters, type UsageReport } from "@/surfaces/comparison/api";

export type UsageResult = { report: UsageReport; groups?: UsageReport["groups"]; seriesError?: string; error?: never }
  | { report?: never; groups?: never; seriesError?: never; error: string };

export function useUsageReport(
  scope: ProjectScope, filters: UsageFilters, after: string | null, revision: number,
  { enabled = true, allDays = false } = {},
) {
  // A response belongs to exactly one scope, set of filters, cursor, and refresh.
  const key = JSON.stringify([scope, filters, after, revision, allDays]);
  const [result, setResult] = useState<{ key: string; data: UsageResult }>();

  useEffect(() => {
    if (!enabled) return;
    const controller = new AbortController();
    const signal = controller.signal;
    let firstPage: UsageReport | undefined;

    async function load() {
      const report = await comparisonApi.usage(scope, filters, after, signal);
      const groups = [...report.groups];
      if (allDays && report.next_after && !signal.aborted) {
        // A 92-day window can span two gateway pages. Keep the report used for
        // export intact and collect the full series independently of the table.
        firstPage = report;
        setResult({ key, data: { report } });
        const visited = new Set<string>();
        for (let cursor: string | null = report.next_after; cursor; ) {
          if (visited.has(cursor)) throw new Error("Repeated usage cursor");
          visited.add(cursor);
          const page = await comparisonApi.usage(scope, filters, cursor, signal);
          groups.push(...page.groups);
          cursor = page.next_after;
        }
      }
      if (!signal.aborted) setResult({ key, data: { report, groups } });
    }

    void load().catch((failure) => {
      if (signal.aborted) return;
      const message = errorMessage(failure);
      setResult({ key, data: firstPage ? { report: firstPage, seriesError: message } : { error: message } });
    });
    return () => controller.abort();
  }, [scope, filters, after, revision, key, enabled, allDays]);

  return enabled && result?.key === key ? result.data : undefined;
}
