import type { ProjectScope } from "@/lib/gateway";
import { ChatHistoryPanel } from "@/surfaces/requests/ChatHistoryPanel";
import { RequestResult } from "@/surfaces/requests/RequestResult";
import { formatProviderCharge, type Trace } from "./api";

export function ProviderTrace({ trace, scope }: { trace: Trace; scope: ProjectScope }) {
  const upstream = trace.upstream;
  if (!upstream) return null;
  const { temperature, top_p } = trace.chat_options;
  const tokens = [upstream.input_tokens, upstream.output_tokens].map((count) => count?.toLocaleString() ?? "—");
  return (
    <div className="mt-3 space-y-3">
      <dl className="grid grid-cols-2 gap-3 rounded-xl bg-surface2/60 p-4 text-xs">
        {(upstream.input_tokens !== null || upstream.output_tokens !== null) && <div><dt className="text-muted">Input / output tokens</dt><dd className="mt-1 font-mono">{tokens.join(" / ")}</dd></div>}
        {upstream.queue_state && <div><dt className="text-muted">Provider queue</dt><dd className="mt-1 capitalize">{upstream.queue_state.replaceAll("_", " ")}</dd></div>}
        {upstream.provider_charge && <div><dt className="text-muted">Provider charge</dt><dd className="mt-1 break-words font-mono">{formatProviderCharge(upstream.provider_charge)}</dd></div>}
        {upstream.finish_reason && <div><dt className="text-muted">Finish reason</dt><dd className="mt-1 break-all font-mono">{upstream.finish_reason}</dd></div>}
        {temperature !== undefined && <div><dt className="text-muted">Temperature</dt><dd className="mt-1 font-mono">{temperature}</dd></div>}
        {top_p !== undefined && <div><dt className="text-muted">Top p</dt><dd className="mt-1 font-mono">{top_p}</dd></div>}
      </dl>
      <ChatHistoryPanel
        key={JSON.stringify([scope.account_id, scope.project_id, trace.id, trace.content_removed_at])}
        scope={scope} id={trace.id} status={trace.status}
        fallback={upstream.result_expires_at && !trace.content_removed_at && (
          <RequestResult key={upstream.result_expires_at} scope={scope} id={trace.id} expiresAt={upstream.result_expires_at} />
        )}
      />
    </div>
  );
}
