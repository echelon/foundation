import { useCallback, useEffect, useState } from "react";
import { comparisonApi, errorMessage, type TracePage } from "@/surfaces/comparison/api";
import type { ProjectScope } from "@/lib/gateway";
import { DIAGNOSTIC_PAGE_SIZE, scanRequestHistory, type DiagnosticScan, type InvestigationFilters } from "./historyDiagnostics";

type Position = { filters: InvestigationFilters } & ({ cursors: string[] } | { matchPage: number });

type HistoryRead = {
  filters: InvestigationFilters;
  after: string | null;
  revision: number;
  page?: TracePage;
  scan?: DiagnosticScan;
  loading: boolean;
  error: string;
};

export function useRequestHistory(scope: ProjectScope, filters: InvestigationFilters) {
  const [position, setPosition] = useState<Position>();
  const [revision, setRevision] = useState(0);
  const [result, setResult] = useState<HistoryRead>();
  const refresh = useCallback(() => setRevision((version) => version + 1), []);
  const currentPosition = position?.filters === filters ? position : undefined;
  const cursors = currentPosition && "cursors" in currentPosition ? currentPosition.cursors : [];
  const after = cursors.at(-1) ?? null;
  const requestedMatchPage = currentPosition && "matchPage" in currentPosition ? currentPosition.matchPage : 0;
  const current = result?.filters === filters && result.after === after ? result : undefined;
  const latest = current?.revision === revision ? current : undefined;
  const loading = latest?.loading ?? true;

  useEffect(() => {
    const controller = new AbortController();
    const { signal } = controller;
    const { diagnostic, ...historyFilters } = filters;
    const initial = { filters, after, revision, loading: true, error: "" };

    function update(patch: Partial<HistoryRead>) {
      if (!signal.aborted) {
        setResult((previous) => {
          const read = previous?.filters === filters && previous.after === after && previous.revision === revision
            ? previous : initial;
          return { ...read, ...patch };
        });
      }
    }

    const read = diagnostic
      ? scanRequestHistory(scope, diagnostic, historyFilters, signal, (scan) => update({ scan }))
      : comparisonApi.history(scope, historyFilters, after, signal).then((page) => update({ page }));
    void read.catch((cause) => update({ error: errorMessage(cause) })).finally(() => update({ loading: false }));
    return () => controller.abort();
  }, [scope, filters, after, revision]);

  const scan = latest?.scan;
  const matching = scan?.matches.length ?? 0;
  const matchPage = Math.min(requestedMatchPage, Math.max(0, Math.ceil(matching / DIAGNOSTIC_PAGE_SIZE) - 1));
  const start = matchPage * DIAGNOSTIC_PAGE_SIZE;
  const matchRows = scan?.matches.slice(start, start + DIAGNOSTIC_PAGE_SIZE) ?? [];
  const nextAfter = current?.page?.next_after;
  const page = filters.diagnostic ? matchPage + 1 : cursors.length + 1;
  const hasOlder = filters.diagnostic ? matching > start + DIAGNOSTIC_PAGE_SIZE : Boolean(nextAfter) && !loading;
  const hasNewer = page > 1 && (Boolean(filters.diagnostic) || !loading);

  return {
    history: current?.page,
    rows: filters.diagnostic ? matchRows.map(({ request }) => request) : current?.page?.requests ?? [],
    progress: scan && { checked: scan.checked, matching, unavailable: scan.unavailable },
    warningIds: new Set(matchRows.filter(({ warning }) => warning).map(({ request }) => request.id)),
    error: latest?.error ?? "",
    loading,
    page,
    hasOlder,
    hasNewer,
    older() {
      if (!hasOlder) return;
      if (filters.diagnostic) setPosition({ filters, matchPage: matchPage + 1 });
      else if (nextAfter) setPosition({ filters, cursors: [...cursors, nextAfter] });
    },
    newer() {
      if (!hasNewer) return;
      if (filters.diagnostic) setPosition({ filters, matchPage: matchPage - 1 });
      else setPosition({ filters, cursors: cursors.slice(0, -1) });
    },
    newest() { setPosition(undefined); },
    refresh,
  };
}
