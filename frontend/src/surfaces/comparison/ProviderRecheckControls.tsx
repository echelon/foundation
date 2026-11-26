import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui";
import type { ProjectScope } from "@/lib/gateway";
import { comparisonApi, errorMessage, type ProviderRecheck } from "./api";

export function ProviderRecheckControls({ scope, requestId, recheck, canSchedule, onRefresh }: {
  scope: ProjectScope;
  requestId: string;
  recheck: ProviderRecheck;
  canSchedule: boolean;
  onRefresh: () => void;
}) {
  const active = useRef<AbortController | null>(null);
  const [saving, setSaving] = useState(false);
  const [scheduled, setScheduled] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => () => active.current?.abort(), []);

  async function schedule() {
    if (active.current) return;
    const controller = new AbortController();
    active.current = controller;
    setSaving(true);
    setError("");
    try {
      await comparisonApi.scheduleRecheck(scope, requestId, controller.signal);
      if (controller.signal.aborted) return;
      setScheduled(true);
      onRefresh();
    } catch (cause) {
      if (!controller.signal.aborted) {
        setError(errorMessage(cause));
        onRefresh();
      }
    } finally {
      active.current = null;
      if (!controller.signal.aborted) setSaving(false);
    }
  }

  if (recheck.state === "finished") {
    return <p className="mb-4 text-sm text-muted">The provider recheck ended. The outcome remains unconfirmed.</p>;
  }
  if (recheck.state === "pending" || scheduled) {
    return <p role="status" className="mb-4 text-sm text-muted">
      Provider recheck scheduled.{" "}
      {recheck.state === "pending" && <>
        Workers can read the original job until{" "}
        <time dateTime={recheck.deadline}>{new Date(recheck.deadline).toLocaleString()}</time>.
      </>}
    </p>;
  }
  if (!canSchedule || Date.parse(recheck.until) <= Date.now()) return null;
  return (
    <div className="mb-4 flex flex-wrap items-center gap-3">
      <p className="text-sm text-muted">Check status and results for the original provider job.</p>
      <Button size="sm" variant="outline" onClick={() => void schedule()} disabled={saving}>
        {saving ? "Scheduling recheck…" : "Recheck provider job"}
      </Button>
      {error && <p role="alert" className="basis-full text-sm text-rose-600 dark:text-rose-300">{error}</p>}
    </div>
  );
}
