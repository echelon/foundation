import { Box, FileAudio, FileImage, FileText, Film, LoaderCircle } from "lucide-react";
import type { ProjectScope } from "@/lib/gateway";
import { formatNanoUsd, usd, type Trace } from "@/surfaces/comparison/api";
import { modalityNames, type Modality } from "@/surfaces/requests/modelOptions";
import { RequestResult } from "@/surfaces/requests/RequestResult";

const modalityIcons = { text: FileText, image: FileImage, video: Film, audio: FileAudio, mesh: Box };

export function PlaygroundOutput({ scope, trace, requestId, submitting, selectedKind, statusError }: {
  scope: ProjectScope; trace?: Trace; requestId: string | null;
  submitting: boolean; selectedKind?: Modality; statusError?: string;
}) {
  if (submitting) return <OutputState pending title="Submitting request" />;
  if (!trace) {
    if (statusError) return <OutputState title="Request status unavailable" />;
    if (requestId) return <OutputState pending title="Reading request" />;
    return <OutputState title="Run a model to see its output" kind={selectedKind} />;
  }
  if (trace.content_removed_at) return <OutputState title="Stored output was removed" />;
  switch (trace.status) {
    case "queued":
      return <OutputState pending title="Queued for execution" details="The waterfall updates as work is dispatched." />;
    case "running":
      return <OutputState pending title="Generating output" details="Follow the waterfall for recorded steps." />;
    case "failed":
      return <OutputState title="Request failed" details="Open the failed step in the waterfall for error details." />;
    case "canceled":
      return <OutputState title="Request canceled" />;
    case "awaiting_reconciliation":
      return <OutputState title="Outcome unconfirmed" details="The gateway will update the request if it receives a confirmed outcome." />;
  }

  const fixture = trace.mock_result;
  const cost = trace.supplier_settlement
    ? ["Settled cost", formatNanoUsd(trace.supplier_settlement.amount_usd_nanos)]
    : trace.upstream?.reported_usd_nanos
      ? ["Reported cost", formatNanoUsd(trace.upstream.reported_usd_nanos)]
      : fixture && trace.mock_reported_usd_nanos !== null
        ? ["Fixture cost", usd(trace.mock_reported_usd_nanos)]
        : ["Quoted estimate", usd(trace.estimated_usd_nanos)];
  return (
    <div className="space-y-4">
      <dl className="flex flex-wrap gap-x-6 gap-y-2 border-b border-border pb-3 text-xs">
        <div><dt className="text-muted">Model</dt><dd className="mt-0.5 break-all font-mono">{trace.model}</dd></div>
        <div><dt className="text-muted">{cost[0]}</dt><dd className="mt-0.5 font-mono">{cost[1]}</dd></div>
      </dl>
      {trace.upstream?.result_expires_at ? (
        <RequestResult key={trace.id + trace.upstream.result_expires_at} scope={scope} id={trace.id} expiresAt={trace.upstream.result_expires_at} autoLoad />
      ) : fixture ? <FixtureResult result={fixture} /> : <p className="text-sm text-muted">No stored output is available for this request.</p>}
    </div>
  );
}

function FixtureResult({ result }: { result: NonNullable<Trace["mock_result"]> }) {
  const details = result.kind === "text" ? result.text
    : result.kind === "image" ? `${result.count} image${result.count === 1 ? "" : "s"} · ${result.aspect_ratio}`
    : result.kind === "video" ? `${result.duration_seconds}s · ${result.resolution} · ${result.aspect_ratio}`
    : result.kind === "audio" ? `${result.duration_seconds}s · ${result.format.toUpperCase()}`
    : result.format.toUpperCase();
  const Icon = modalityIcons[result.kind];
  return (
    <div className="rounded-lg border border-border bg-surface2/50 p-4">
      <p className="flex items-center gap-2 text-xs font-medium text-muted">
        <Icon className="h-4 w-4" aria-hidden />{modalityNames[result.kind]} fixture
      </p>
      <p className="mt-3 max-h-96 overflow-auto whitespace-pre-wrap break-words font-mono text-sm leading-6">{details}</p>
    </div>
  );
}

function OutputState({ title, details, pending, kind }: { title: string; details?: string; pending?: boolean; kind?: Modality }) {
  const Icon = pending ? LoaderCircle : modalityIcons[kind ?? "text"];
  return (
    <div role="status" className="grid min-h-48 flex-1 place-items-center rounded-xl border border-dashed border-border bg-surface2/40 p-5 text-center">
      <div className="space-y-2">
        <div className="mx-auto grid h-10 w-10 place-items-center rounded-lg border border-border bg-surface">
          <Icon className={pending ? "h-5 w-5 animate-spin text-brand motion-reduce:animate-none" : "h-5 w-5 text-muted"} aria-hidden />
        </div>
        <p className="text-sm font-medium">{title}</p>
        {details && <p className="max-w-xs text-sm text-muted">{details}</p>}
      </div>
    </div>
  );
}
