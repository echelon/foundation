import { useEffect, useRef, useState } from "react";
import { Button, Card } from "@/components/ui";
import { ApiError } from "@/lib/api";
import { downloadBlob } from "@/lib/download";
import { errorMessage, gatewayRequest, gatewayResponse, type ProjectScope } from "@/lib/gateway";

const statusLabels = {
  queued: "Queued",
  sealing: "Sealing",
  stored: "Stored",
  restore_queued: "Restore queued",
  restoring: "Verifying restore",
  retrieving: "Waiting for object storage",
  available: "Ready to download",
  archive_failed: "Archive failed",
  restore_failed: "Restore failed",
};
type Status = keyof typeof statusLabels;
type Archive = {
  id: string;
  export_id: string;
  client_ref: string | null;
  state: Status;
  event_count: number | null;
  last_event_id: string | null;
  has_more: boolean | null;
  content_sha256: string | null;
  restored_until: string | null;
  can_request_retrieval: boolean;
  error_code: string | null;
  automatic?: boolean;
  automatic_retry_at?: string;
  retired_at?: string;
  retirement_error_code?: string;
};
type Page = { archives: Archive[]; next_before: string | null };
const active = new Set<Status>(["queued", "sealing", "restore_queued", "restoring"]);
const canRestore = new Set<Status>(["stored", "restore_failed"]);
const archiveErrors: Readonly<Record<string, string>> = {
  archive_retrieval_timed_out: "Check the retrieval in your storage console, then use Restore archive to check again.",
  archive_retrieval_rejected: "Object storage declined the retrieval. Check the bucket’s restore permissions.",
  archive_partition_incomplete: "The scheduled range has missing events. Check the project’s export history.",
  archive_source_retrieval_required: "Restore the scheduled archive covering this range, then queue the seal again.",
  archive_source_unavailable: "A source archive is unavailable. Check archive storage before retrying.",
};

export function Archives({ scope, exportId }: { scope: ProjectScope; exportId?: string }) {
  const [page, setPage] = useState<Page>();
  const [before, setBefore] = useState<string>();
  const [enabled, setEnabled] = useState<boolean>();
  const [refresh, setRefresh] = useState(0);
  const [busy, setBusy] = useState(false);
  const [issue, setIssue] = useState("");
  const action = useRef<AbortController | null>(null);

  useEffect(() => () => action.current?.abort(), []);
  useEffect(() => {
    const request = new AbortController();
    const params = new URLSearchParams(scope);
    if (before) params.set("before", before);
    let timer: number | undefined;
    async function load() {
      try {
        const next = await gatewayRequest<Page>(`/v1/archives?${params}`, { signal: request.signal });
        if (request.signal.aborted) return;
        setPage(next);
        setEnabled(true);
        if (next.archives.some((archive) => active.has(archive.state))) {
          timer = window.setTimeout(() => void load(), 800);
        } else {
          const now = Date.now();
          const changes = next.archives.flatMap((archive) => [archive.restored_until, archive.automatic_retry_at])
            .map((time) => time ? Date.parse(time) : 0).filter((time) => time > now);
          const untilChange = Math.min(...changes) - now + 250;
          const untilRetrievalCheck = next.archives.some((archive) => archive.state === "retrieving") ? 15_000 : Infinity;
          const delay = Math.min(untilChange, untilRetrievalCheck);
          if (Number.isFinite(delay)) timer = window.setTimeout(() => void load(), Math.max(800, delay));
        }
      } catch (error) {
        if (request.signal.aborted) return;
        if (error instanceof ApiError && error.status === 404) setEnabled(false);
        else { setEnabled(true); setIssue(errorMessage(error)); }
      }
    }
    void load();
    return () => { request.abort(); window.clearTimeout(timer); };
  }, [scope, before, refresh]);

  async function run(operation: (signal: AbortSignal) => Promise<void>) {
    if (action.current) return;
    const request = new AbortController();
    action.current = request;
    setBusy(true);
    setIssue("");
    try {
      await operation(request.signal);
      if (!request.signal.aborted) setRefresh((value) => value + 1);
    } catch (error) {
      if (!request.signal.aborted) setIssue(errorMessage(error));
    } finally {
      action.current = null;
      if (!request.signal.aborted) setBusy(false);
    }
  }

  function queue(id: string, after?: string) {
    void run(async (signal) => {
      await gatewayRequest("/v1/archives", {
        method: "POST", headers: { "Content-Type": "application/json" }, signal,
        body: JSON.stringify({ ...scope, export_id: id, after }),
      });
      if (!signal.aborted) setBefore(undefined);
    });
  }

  function restore(id: string, retrieve = false) {
    void run(async (signal) => {
      const params = new URLSearchParams(scope);
      if (retrieve) params.set("retrieve", "true");
      await gatewayRequest(`/v1/archives/${encodeURIComponent(id)}/restore?${params}`, {
        method: "POST", signal,
        headers: retrieve ? { "Content-Type": "application/json" } : undefined,
      });
    });
  }

  function download(id: string) {
    void run(async (signal) => {
      const response = await gatewayResponse(`/v1/archives/${encodeURIComponent(id)}/data?${new URLSearchParams(scope)}`, { signal });
      const blob = await response.blob();
      if (!signal.aborted) downloadBlob(blob, `tokencraft-${id}.ndjson`);
    });
  }

  if (!enabled) return null;
  return (
    <Card className="space-y-3 p-4 sm:p-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="font-display text-xl font-semibold">Archives</h2>
          <p className="mt-1 text-sm text-muted">Seal export events for storage and restore a verified NDJSON download when needed.</p>
        </div>
        {exportId && <Button type="button" size="sm" variant="outline" disabled={busy} onClick={() => queue(exportId)}>Archive selected export</Button>}
      </div>
      {issue && <p className="text-sm text-rose-600 dark:text-rose-300" role="alert">{issue}</p>}
      {page?.archives.length === 0 && <p className="text-sm text-muted">Open an export to create its first archive.</p>}
      <ul className="divide-y divide-border">
        {page?.archives.map((archive) => {
          const expiry = archive.restored_until ? Date.parse(archive.restored_until) : 0;
          const state = archive.state === "available" && expiry <= Date.now() ? "stored" : archive.state;
          const needsRetrieval = archive.error_code === "archive_retrieval_required";
          const canRetrieve = needsRetrieval && archive.can_request_retrieval;
          const error = archive.error_code;
          const notice = needsRetrieval
            ? canRetrieve
              ? "Retrieval can take hours. Your storage provider may charge for it."
              : "Request retrieval in your object storage console. Once the object is available, choose Restore archive again."
            : error && (archiveErrors[error] ?? error.replaceAll("_", " "));
          return (
            <li key={archive.id} className="space-y-2 py-3">
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div className="min-w-0">
                  <p className="break-all text-sm font-medium">
                    {archive.automatic ? "Scheduled archive · All clients" : archive.client_ref ?? "All clients"}
                    {archive.event_count && ` · ${archive.event_count} ${archive.event_count === 1 ? "event" : "events"}`}
                  </p>
                  <p className="break-all font-mono text-xs text-muted">{archive.id}</p>
                  <p className="mt-1 text-xs text-muted" role="status">{needsRetrieval ? "Retrieval required" : statusLabels[state]}</p>
                </div>
                <div className="flex flex-wrap gap-2">
                  {canRestore.has(state) && <Button type="button" size="sm" variant="outline" disabled={busy} onClick={() => restore(archive.id)}>Restore archive</Button>}
                  {canRetrieve && <Button type="button" size="sm" disabled={busy} onClick={() => restore(archive.id, true)}>Request cold retrieval</Button>}
                  {state === "available" && <Button type="button" size="sm" disabled={busy} onClick={() => download(archive.id)}>Download restored NDJSON</Button>}
                  {archive.has_more && archive.last_event_id && archive.export_id === exportId &&
                    <Button type="button" size="sm" variant="outline" disabled={busy} onClick={() => queue(archive.export_id, archive.last_event_id ?? undefined)}>Archive next partition</Button>}
                </div>
              </div>
              {notice && <p className="text-sm text-muted">{notice}</p>}
              {archive.automatic_retry_at && <p className="text-xs text-muted">
                Retry eligible at <time dateTime={archive.automatic_retry_at}>{new Date(archive.automatic_retry_at).toLocaleString()}</time>
              </p>}
              {archive.retired_at && <p className="text-xs text-muted">
                Event payloads moved to archive <time dateTime={archive.retired_at}>{new Date(archive.retired_at).toLocaleString()}</time>.
              </p>}
              {archive.retirement_error_code && <p className="text-xs text-muted">
                Retirement deferred · {archive.retirement_error_code.replaceAll("_", " ")}
              </p>}
              {state === "available" && archive.restored_until && <p className="text-xs text-muted">Available until <time dateTime={archive.restored_until}>{new Date(archive.restored_until).toLocaleString()}</time></p>}
              {archive.content_sha256 && <p className="break-all text-xs text-muted">SHA-256 <code>{archive.content_sha256}</code></p>}
            </li>
          );
        })}
      </ul>
      <div className="flex flex-wrap gap-2">
        <Button type="button" size="sm" variant="outline" onClick={() => { setIssue(""); setRefresh((value) => value + 1); }}>Refresh archives</Button>
        {before && <Button type="button" size="sm" variant="outline" onClick={() => setBefore(undefined)}>Newest archives</Button>}
        {page?.next_before && <Button type="button" size="sm" variant="outline" onClick={() => setBefore(page.next_before ?? undefined)}>Older archives</Button>}
      </div>
    </Card>
  );
}
