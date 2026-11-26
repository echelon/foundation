import type { ProjectScope } from "@/lib/gateway";
import { comparisonApi, type HistoryFilters, type Trace } from "@/surfaces/comparison/api";

export const diagnosticNames = {
  warnings: "Has warnings",
  failed_or_warning: "Failed or has warnings",
} as const;

export type Diagnostic = keyof typeof diagnosticNames;
export type InvestigationFilters = HistoryFilters & { diagnostic?: Diagnostic };

export function diagnosticFromUrl(value: string | null): Diagnostic | undefined {
  return value === "warnings" || value === "failed_or_warning" ? value : undefined;
}

export function validDate(value: string) {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value) || value.startsWith("0000")) return false;
  const time = new Date(value + "T00:00:00Z");
  return Number.isFinite(time.getTime()) && time.toISOString().slice(0, 10) === value;
}

export function dateRangeError({ from, to }: HistoryFilters) {
  if (!from && !to) return "";
  if (!from || !to || !validDate(from) || !validDate(to)) return "Enter both UTC dates.";
  const days = (Date.parse(to ?? "") - Date.parse(from ?? "")) / 86_400_000;
  return days >= 1 && days <= 92 ? "" : "Choose a UTC date range of 1 to 92 days.";
}

export function shiftDate(date: string, days: number) {
  if (!validDate(date)) return "";
  const time = new Date(date + "T00:00:00Z");
  time.setUTCDate(time.getUTCDate() + days);
  return time.toISOString().slice(0, 10);
}

export const DIAGNOSTIC_PAGE_SIZE = 25;

type DiagnosticMatch = { request: Trace; warning: boolean };

export type DiagnosticScan = {
  matches: DiagnosticMatch[];
  checked: number;
  unavailable: number;
};

export async function scanRequestHistory(
  scope: ProjectScope, diagnostic: Diagnostic, filters: HistoryFilters, signal: AbortSignal,
  onProgress: (progress: DiagnosticScan) => void,
) {
  let after: string | null = null;
  const cursors = new Set<string>();
  let checked = 0;
  let unavailable = 0;
  let matches: DiagnosticMatch[] = [];

  do {
    const page = await comparisonApi.history(scope, filters, after, signal);
    if (page.next_after && cursors.has(page.next_after)) throw new Error("The history cursor did not advance.");
    const pageMatches: DiagnosticMatch[] = [];
    for (let index = 0; index < page.requests.length; index += 5) {
      signal.throwIfAborted();
      const evidence = await Promise.all(page.requests.slice(index, index + 5).map(async (request) => {
        if (diagnostic === "failed_or_warning" && request.status === "failed") {
          return { request, warning: false };
        }
        try {
          const trace = await comparisonApi.debugTrace(scope, request.id, signal);
          if (trace.evidence.steps.some((step) =>
            step.tone === "warning" || step.logs.some((log) => log.severity === "warning"))) {
            return { request, warning: true };
          }
        } catch {
          if (!signal.aborted) unavailable += 1;
        }
      }));
      for (const match of evidence) {
        if (match) pageMatches.push(match);
      }
    }

    signal.throwIfAborted();
    checked += page.requests.length;
    after = page.next_after;
    // The gateway sorts newest first, so later pages can only append older matches.
    if (matches.length === 0) matches = pageMatches;
    else if (pageMatches.length > 0) matches = [...matches, ...pageMatches];
    onProgress({ matches, checked, unavailable });
    if (after) cursors.add(after);
  } while (after);
}
