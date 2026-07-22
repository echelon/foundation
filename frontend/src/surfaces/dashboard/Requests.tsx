/**
 * Dashboard → Requests: the organization's REAL generation history (`GET /v1/jobs`),
 * newest first with keyset "load more". Each row is a job: outputs (CDN thumbnails),
 * status + fan-out progress, model, and the price estimate.
 */

import { useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { Badge, Button, Card, Select } from "@/components/ui";
import { BatchBadge } from "@/components/BatchBadge";
import { OutputLightbox, sessionRequestLoader } from "@/components/OutputLightbox";
import { useOrg } from "@/lib/org";
import { api, type ApiJob } from "@/lib/api";
import { MODEL_BY_SLUG, MODELS } from "@/lib/catalog";

const LIVE_MODELS = MODELS.filter((m) => m.live);

export default function Requests() {
  const { current, loading: orgLoading } = useOrg();
  const [model, setModel] = useState<string>("all");
  const [jobs, setJobs] = useState<ApiJob[]>([]);
  const [nextAfter, setNextAfter] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [nonce, setNonce] = useState(0);
  const [selected, setSelected] = useState<{ job: ApiJob; index: number } | null>(null);

  useEffect(() => {
    if (!current) return;
    let dead = false;
    setLoading(true);
    api
      .listJobs(current.account_id, model === "all" ? undefined : { model })
      .then((page) => {
        if (dead) return;
        setJobs(page.jobs);
        setNextAfter(page.next_after);
      })
      .catch(() => {})
      .finally(() => setLoading(false));
    return () => {
      dead = true;
    };
  }, [current, model, nonce]);

  const loadMore = async () => {
    if (!current || !nextAfter) return;
    setLoading(true);
    try {
      const page = await api.listJobs(current.account_id, {
        after: nextAfter,
        ...(model === "all" ? {} : { model }),
      });
      setJobs((j) => [...j, ...page.jobs]);
      setNextAfter(page.next_after);
    } finally {
      setLoading(false);
    }
  };

  const totalEstimate = jobs.reduce((s, j) => s + j.estimated_micros, 0);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-3xl font-bold tracking-tight">Requests</h1>
        <p className="mt-1 text-muted">Every generation job on the organization — playground, dashboard, and API.</p>
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <Select value={model} onChange={(e) => setModel(e.target.value)}>
          <option value="all">All models</option>
          {LIVE_MODELS.map((m) => (
            <option key={m.slug} value={m.slug}>
              {m.name}
            </option>
          ))}
        </Select>
        <Button variant="outline" size="sm" onClick={() => setNonce((n) => n + 1)} disabled={loading}>
          <RefreshCw className="h-4 w-4" /> Refresh
        </Button>
        {jobs.length > 0 && (
          <span className="ml-auto text-sm text-muted">
            {jobs.length} job{jobs.length === 1 ? "" : "s"} · est. ${(totalEstimate / 1_000_000).toFixed(4)}
          </span>
        )}
      </div>

      <Card className="overflow-hidden">
        {orgLoading || (loading && jobs.length === 0) ? (
          <p className="px-5 py-8 text-sm text-muted">Loading…</p>
        ) : jobs.length === 0 ? (
          <p className="px-5 py-8 text-sm text-muted">
            No jobs yet. Run something in a model playground or hit the API — it lands here.
          </p>
        ) : (
          <div className="scrollbar-thin overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border text-left text-xs text-muted">
                  <th className="py-2 pl-5 pr-3 font-medium">Output</th>
                  <th className="px-3 py-2 font-medium">Job</th>
                  <th className="px-3 py-2 font-medium">Model</th>
                  <th className="px-3 py-2 font-medium">Source</th>
                  <th className="px-3 py-2 font-medium">Status</th>
                  <th className="hidden px-3 py-2 font-medium sm:table-cell">Kind</th>
                  <th className="py-2 pl-3 pr-5 text-right font-medium">Est. cost</th>
                </tr>
              </thead>
              <tbody>
                {jobs.map((j) => (
                  <tr
                    key={j.id}
                    onClick={() => setSelected({ job: j, index: 0 })}
                    title="View details"
                    className="cursor-pointer border-b border-border/60 last:border-0 hover:bg-surface2/40"
                  >
                    <td className="py-3 pl-5 pr-3">
                      <div className="flex items-center gap-2">
                        <div className="flex -space-x-2">
                          {j.outputs.slice(0, 3).map((o, i) => (
                            <button
                              key={o.index}
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                setSelected({ job: j, index: i });
                              }}
                            >
                              <img src={o.url} alt="" className="h-10 w-10 rounded-lg border border-border object-cover" />
                            </button>
                          ))}
                          {j.outputs.length === 0 && <div className="h-10 w-10 rounded-lg bg-surface2" />}
                        </div>
                        <BatchBadge count={j.progress?.total ?? j.outputs.length} className="relative z-10 -ml-3.5 ring-2 ring-bg" />
                      </div>
                    </td>
                    <td className="px-3 py-3">
                      <div className="font-mono text-[13px] text-fg">{j.id}</div>
                      <div className="text-xs text-muted">{new Date(j.created_at).toLocaleString()}</div>
                    </td>
                    <td className="px-3 py-3 font-mono text-xs text-muted">{j.model}</td>
                    <td className="px-3 py-3">
                      <div className="flex flex-col gap-1">
                        <Badge tone={j.source === "api_key" ? "brand" : "muted"}>
                          {j.source === "api_key" ? "API key" : j.source}
                        </Badge>
                        <span className="font-mono text-[11px] text-muted">
                          {j.api_shape === "any" ? "/v1/any/image" : "/v1/models/{slug}"}
                        </span>
                      </div>
                    </td>
                    <td className="px-3 py-3">
                      <JobStatusBadge job={j} />
                    </td>
                    <td className="hidden px-3 py-3 text-xs text-muted sm:table-cell">{j.modality.replace(/_/g, " ")}</td>
                    <td className="py-3 pl-3 pr-5 text-right font-medium text-fg">
                      ${(j.estimated_micros / 1_000_000).toFixed(4)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {nextAfter && (
          <div className="border-t border-border px-5 py-3">
            <Button variant="outline" size="sm" onClick={() => void loadMore()} disabled={loading}>
              {loading ? "Loading…" : "Load more"}
            </Button>
          </div>
        )}
      </Card>

      {selected && current && (
        <OutputLightbox
          job={selected.job}
          modelName={MODEL_BY_SLUG[selected.job.model]?.name ?? selected.job.model}
          initialIndex={selected.index}
          loadRequest={sessionRequestLoader(current.account_id)}
          onClose={() => setSelected(null)}
        />
      )}
    </div>
  );
}

export function JobStatusBadge({ job }: { job: ApiJob }) {
  const badge =
    job.status === "succeeded" ? (
      <Badge tone="green">succeeded</Badge>
    ) : job.status === "failed" ? (
      <Badge tone="red">failed</Badge>
    ) : job.status === "running" ? (
      <Badge tone="amber">running</Badge>
    ) : job.status === "queued" ? (
      <Badge tone="brand">queued</Badge>
    ) : (
      <Badge>{job.status}</Badge>
    );
  return (
    <span className="inline-flex items-center gap-2">
      {badge}
      {job.progress && (
        <span className="text-xs text-muted">
          {job.progress.succeeded}/{job.progress.total}
        </span>
      )}
    </span>
  );
}
