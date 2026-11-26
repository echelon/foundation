import { useEffect, useRef, useState } from "react";
import { Download } from "lucide-react";
import { Button } from "@/components/ui";
import { downloadBlob } from "@/lib/download";
import { errorMessage, gatewayResponse, isGatewayEndpointUnavailable, withProject, type ProjectScope } from "@/lib/gateway";
import type { TraceSnapshot } from "@/surfaces/comparison/api";

function downloadTiming(snapshot: TraceSnapshot) {
  const { evidence, revision, observed_at } = snapshot;
  const timing = {
    request_id: evidence.request_id,
    status: evidence.status,
    revision,
    observed_at,
    steps: evidence.steps.map(({ id, parent_id, component, kind, started_at, ended_at, tone, logs }) => ({
      id, parent_id, component, kind, started_at, ended_at, tone,
      events: logs.map(({ at, code }) => ({ at, code })),
    })),
  };
  const blob = new Blob([JSON.stringify(timing, null, 2)], { type: "application/json" });
  downloadBlob(blob, `tokencraft-${evidence.request_id}-timing.json`);
}

export function TraceDownload({ scope, snapshot }: { scope: ProjectScope; snapshot: TraceSnapshot }) {
  const requestId = snapshot.evidence.request_id;
  const active = useRef<AbortController | null>(null);
  const [pending, setPending] = useState(false);
  const [timingOnly, setTimingOnly] = useState(false);
  const [notice, setNotice] = useState("");
  useEffect(() => () => active.current?.abort(), []);

  async function download() {
    if (active.current) return;
    if (timingOnly) {
      downloadTiming(snapshot);
      setNotice("Request timings downloaded.");
      return;
    }
    const controller = new AbortController();
    active.current = controller;
    setPending(true);
    setNotice("");
    try {
      const path = withProject(`/v1/requests/${encodeURIComponent(requestId)}/trace/otlp`, scope.project_id, scope.account_id);
      const response = await gatewayResponse(path, { signal: controller.signal });
      const blob = await response.blob();
      if (!controller.signal.aborted) {
        downloadBlob(blob, `tokencraft-${requestId}-otel.json`);
        setNotice("OpenTelemetry trace downloaded.");
      }
    } catch (cause) {
      if (controller.signal.aborted) return;
      if (isGatewayEndpointUnavailable(cause)) {
        setTimingOnly(true);
        downloadTiming(snapshot);
        setNotice("Downloaded request timings. OpenTelemetry export is unavailable on this gateway.");
      } else {
        setNotice(errorMessage(cause));
      }
    } finally {
      active.current = null;
      if (!controller.signal.aborted) setPending(false);
    }
  }

  return (
    <>
      <Button variant="outline" size="sm" onClick={() => void download()} disabled={pending}
        aria-label={timingOnly ? "Download request timings" : "Download OpenTelemetry trace"}>
        <Download className="h-4 w-4" aria-hidden /> {pending ? "Downloading…" : timingOnly ? "Download request timings" : "Download trace"}
      </Button>
      <p className="basis-full text-xs text-muted empty:hidden" role="status">{notice}</p>
    </>
  );
}
