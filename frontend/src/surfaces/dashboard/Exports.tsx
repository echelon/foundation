import { useEffect, useRef, useState, type FormEvent } from "react";
import { Button, Card, Input } from "@/components/ui";
import { errorMessage, gatewayRequest, gatewayResponse, type ProjectScope } from "@/lib/gateway";
import { downloadBlob } from "@/lib/download";
import { ProjectStatus, useProject } from "@/surfaces/projects/ProjectScope";
import { Archives } from "./Archives";

type Export = {
  id: string;
  client_ref: string | null;
  after_event_id: string | null;
  pending_at_creation: boolean;
  created_at: string;
  expires_at: string;
};
type ExportList = { exports: Export[]; next_before: string | null };
type DataPage = { index: number; blob: Blob; rows: number; sha256: string; nextAfter: string | null; hasMore: boolean };

export default function Exports() {
  const { project, scope } = useProject();
  if (!scope || !project) return <ProjectStatus />;
  if (project.role !== "admin") return <p className="text-sm text-muted" role="status">A project administrator can export project events.</p>;
  return <Workspace key={scope.project_id} scope={scope} />;
}

function Workspace({ scope }: { scope: ProjectScope }) {
  const [list, setList] = useState<ExportList>();
  const [selected, setSelected] = useState<Export>();
  const [clientRef, setClientRef] = useState("");
  const [after, setAfter] = useState("");
  const [before, setBefore] = useState<string>();
  const [refresh, setRefresh] = useState(0);
  const [creating, setCreating] = useState(false);
  const [issue, setIssue] = useState("");
  const creatingRequest = useRef<AbortController | null>(null);

  useEffect(() => () => creatingRequest.current?.abort(), []);
  useEffect(() => {
    const request = new AbortController();
    const params = new URLSearchParams(scope);
    if (before) params.set("before", before);
    gatewayRequest<ExportList>(`/v1/exports?${params}`, { signal: request.signal }).then((value) => {
      if (!request.signal.aborted) setList(value);
    }).catch((error) => {
      if (!request.signal.aborted) setIssue(errorMessage(error));
    });
    return () => request.abort();
  }, [scope, before, refresh]);

  async function create(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (creatingRequest.current) return;
    const request = new AbortController();
    creatingRequest.current = request;
    setCreating(true);
    setIssue("");
    try {
      const snapshot = await gatewayRequest<Export>("/v1/exports", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        signal: request.signal,
        body: JSON.stringify({ ...scope, client_ref: clientRef.trim() || null, after: after.trim() || null }),
      });
      if (request.signal.aborted) return;
      setSelected(snapshot);
      setBefore(undefined);
      setRefresh((value) => value + 1);
    } catch (error) {
      if (!request.signal.aborted) setIssue(errorMessage(error));
    } finally {
      if (!request.signal.aborted) setCreating(false);
      creatingRequest.current = null;
    }
  }

  return (
    <section aria-label="Project exports" className="space-y-6">
      <p className="max-w-2xl text-muted">Take a fixed snapshot of request and usage events. Download it as NDJSON or resume from an event cursor.</p>
      <Card className="space-y-4 p-4 sm:p-5">
        <form className="flex flex-wrap items-end gap-3" onSubmit={create}>
          <label className="min-w-0 w-full max-w-56 space-y-1 text-sm font-medium">Client reference
            <Input value={clientRef} onChange={(event) => setClientRef(event.target.value)} maxLength={64} placeholder="All clients" disabled={creating} />
          </label>
          <label className="min-w-0 w-full max-w-xs space-y-1 text-sm font-medium">Resume after event
            <Input value={after} onChange={(event) => setAfter(event.target.value)} maxLength={36} placeholder="Full history" disabled={creating} />
          </label>
          <Button type="submit" disabled={creating}>{creating ? "Creating…" : "Create export"}</Button>
        </form>
        {issue && <p role="alert" className="text-sm text-rose-600 dark:text-rose-300">{issue}</p>}
      </Card>
      {selected && <ExportData key={selected.id} scope={scope} snapshot={selected} />}
      <Archives scope={scope} exportId={selected?.id} />
      <Card className="space-y-3 p-4 sm:p-5">
        <h2 className="font-display text-xl font-semibold">Recent exports</h2>
        {!list && !issue && <p role="status" className="text-sm text-muted">Loading exports…</p>}
        {list?.exports.length === 0 && <p className="text-sm text-muted">There are no exports available.</p>}
        {Boolean(list?.exports.length) && <ul className="divide-y divide-border">
          {list?.exports.map((snapshot) => (
            <li key={snapshot.id} className="flex flex-wrap items-center justify-between gap-2 py-3">
              <div className="min-w-0">
                <p className="break-all text-sm font-medium">{snapshot.client_ref ?? "All clients"}</p>
                <time className="text-xs text-muted" dateTime={snapshot.created_at}>{new Date(snapshot.created_at).toLocaleString()}</time>
              </div>
              <Button type="button" size="sm" variant="outline" onClick={() => setSelected(snapshot)}>Open export</Button>
            </li>
          ))}
        </ul>}
        <div className="flex flex-wrap gap-2">
          {before && <Button type="button" size="sm" variant="outline" onClick={() => { setList(undefined); setBefore(undefined); }}>Newest exports</Button>}
          {list?.next_before && <Button type="button" size="sm" variant="outline" onClick={() => {
            setBefore(list.next_before ?? undefined); setList(undefined);
          }}>Older exports</Button>}
        </div>
      </Card>
    </section>
  );
}

function ExportData({ scope, snapshot }: { scope: ProjectScope; snapshot: Export }) {
  const [cursors, setCursors] = useState<string[]>([]);
  const [response, setResponse] = useState<DataPage>();
  const [issue, setIssue] = useState("");
  const [retry, setRetry] = useState(0);
  const index = cursors.length;
  const after = cursors.at(-1);

  useEffect(() => {
    const request = new AbortController();
    const params = new URLSearchParams(scope);
    if (after) params.set("after", after);
    async function open() {
      try {
        const response = await gatewayResponse(`/v1/exports/${encodeURIComponent(snapshot.id)}/data?${params}`, { signal: request.signal });
        const blob = await response.blob();
        const rows = (await blob.text()).split("\n").filter(Boolean).length;
        if (request.signal.aborted) return;
        setResponse({
          index, blob, rows,
          sha256: response.headers.get("X-TokenCraft-Content-Sha256") ?? "",
          nextAfter: response.headers.get("X-TokenCraft-Next-After"),
          hasMore: response.headers.get("X-TokenCraft-Has-More") === "true",
        });
      } catch (error) {
        if (!request.signal.aborted) setIssue(errorMessage(error));
      }
    }
    void open();
    return () => request.abort();
  }, [scope, snapshot.id, after, index, retry]);

  function download() {
    if (!response) return;
    downloadBlob(response.blob, `tokencraft-${snapshot.id}-page-${index + 1}.ndjson`);
  }

  return (
    <Card className="space-y-3 p-4 sm:p-5">
      <div>
        <h2 className="font-display text-xl font-semibold">Export snapshot</h2>
        <p className="mt-1 text-sm text-muted">{snapshot.client_ref ?? "All clients"} · Expires {new Date(snapshot.expires_at).toLocaleString()}</p>
        <p className="mt-1 break-all font-mono text-xs text-muted">{snapshot.id}</p>
      </div>
      {snapshot.pending_at_creation && <p className="text-sm text-muted">More events were pending when this snapshot was created. Start another export from the last cursor to include them.</p>}
      {issue && <div className="flex flex-wrap items-center gap-2">
        <p role="alert" className="text-sm text-rose-600 dark:text-rose-300">{issue}</p>
        <Button type="button" size="sm" variant="outline" onClick={() => { setIssue(""); setRetry((value) => value + 1); }}>Retry</Button>
      </div>}
      {!response && !issue && <p role="status" className="text-sm text-muted">Loading export page…</p>}
      {response?.index === index && <>
        <p className="text-sm text-muted">Page {index + 1} · {response.rows} {response.rows === 1 ? "event" : "events"}</p>
        {response.nextAfter && <p className="break-all text-xs text-muted">Resume after <code>{response.nextAfter}</code></p>}
        {response.sha256 && <p className="break-all text-xs text-muted">SHA-256 <code>{response.sha256}</code></p>}
        <div className="flex flex-wrap gap-2">
          <Button type="button" size="sm" onClick={download} disabled={!response.rows}>Download NDJSON page</Button>
          {index > 0 && <Button type="button" size="sm" variant="outline" onClick={() => {
            setIssue(""); setResponse(undefined); setCursors((current) => current.slice(0, -1));
          }}>Previous page</Button>}
          {response.hasMore && response.nextAfter && <Button type="button" size="sm" variant="outline" onClick={() => {
            setIssue(""); setResponse(undefined); setCursors([...cursors, response.nextAfter!]);
          }}>Next page</Button>}
        </div>
      </>}
    </Card>
  );
}
