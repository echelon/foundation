/**
 * Internal admin console — recent generation jobs across EVERY account
 * (`GET /v1/admin/jobs`, staff only). A record browser: keyset pagination, no polling.
 */

import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { ArrowLeft, RefreshCw, ShieldAlert } from "lucide-react";
import { Badge, Button, Card, Container, Logo, ThemeToggle } from "@/components/ui";
import { BatchBadge } from "@/components/BatchBadge";
import { OutputThumb } from "@/components/OutputThumb";
import { OutputLightbox } from "@/components/OutputLightbox";
import { api, ApiError, type AdminJob } from "@/lib/api";
import { MODEL_BY_SLUG } from "@/lib/catalog";

export default function Admin() {
  const [jobs, setJobs] = useState<AdminJob[]>([]);
  const [nextAfter, setNextAfter] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [nonce, setNonce] = useState(0);
  const [selected, setSelected] = useState<{ job: AdminJob; index: number } | null>(null);

  useEffect(() => {
    let dead = false;
    setLoading(true);
    setError(null);
    api
      .adminListJobs()
      .then((page) => {
        if (dead) return;
        setJobs(page.jobs);
        setNextAfter(page.next_after);
      })
      .catch((e) => {
        if (dead) return;
        setError(e instanceof ApiError && e.status === 403 ? "staff_only" : "Failed to load jobs.");
      })
      .finally(() => setLoading(false));
    return () => {
      dead = true;
    };
  }, [nonce]);

  const loadMore = async () => {
    if (!nextAfter) return;
    setLoading(true);
    try {
      const page = await api.adminListJobs(nextAfter);
      setJobs((j) => [...j, ...page.jobs]);
      setNextAfter(page.next_after);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-bg text-fg">
      <header className="flex items-center justify-between border-b border-border px-6 py-4">
        <div className="flex items-center gap-3">
          <Logo />
          <Badge tone="amber">Internal</Badge>
        </div>
        <div className="flex items-center gap-2">
          <Link to="/dashboard">
            <Button variant="outline" size="sm">
              <ArrowLeft className="h-4 w-4" /> Dashboard
            </Button>
          </Link>
          <ThemeToggle />
        </div>
      </header>
      <Container className="py-10">
        {error === "staff_only" ? (
          <Card className="mx-auto max-w-lg p-10 text-center">
            <div className="mx-auto mb-5 grid h-14 w-14 place-items-center rounded-2xl bg-amber-500/12 text-amber-500">
              <ShieldAlert className="h-7 w-7" />
            </div>
            <h1 className="font-display text-2xl font-bold tracking-tight">Staff only</h1>
            <p className="mx-auto mt-3 max-w-sm text-muted">
              This is the internal admin console. Your account doesn't have staff access.
            </p>
          </Card>
        ) : (
          <div className="space-y-6">
            <div className="flex flex-wrap items-end justify-between gap-3">
              <div>
                <h1 className="font-display text-3xl font-bold tracking-tight">Recent jobs</h1>
                <p className="mt-1 text-muted">Generation jobs across every account, newest first.</p>
              </div>
              <Button variant="outline" size="sm" onClick={() => setNonce((n) => n + 1)} disabled={loading}>
                <RefreshCw className="h-4 w-4" /> Refresh
              </Button>
            </div>
            {error && <p className="rounded-lg bg-red-500/10 px-4 py-3 text-sm text-red-500">{error}</p>}
            <Card className="overflow-hidden">
              {loading && jobs.length === 0 ? (
                <p className="px-5 py-8 text-sm text-muted">Loading…</p>
              ) : jobs.length === 0 && !error ? (
                <p className="px-5 py-8 text-sm text-muted">No jobs recorded yet.</p>
              ) : (
                <div className="scrollbar-thin overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b border-border text-left text-xs text-muted">
                        <th className="py-2 pl-5 pr-3 font-medium">Output</th>
                        <th className="px-3 py-2 font-medium">Job</th>
                        <th className="px-3 py-2 font-medium">Account</th>
                        <th className="px-3 py-2 font-medium">Model</th>
                        <th className="px-3 py-2 font-medium">Status</th>
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
                                    title="View this result"
                                  >
                                    <OutputThumb output={o} />
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
                          <td className="px-3 py-3 font-mono text-xs text-muted">{j.account_id}</td>
                          <td className="px-3 py-3 font-mono text-xs text-muted">{j.model}</td>
                          <td className="px-3 py-3">
                            {j.status === "succeeded" ? (
                              <Badge tone="green">succeeded</Badge>
                            ) : j.status === "failed" ? (
                              <Badge tone="red">failed</Badge>
                            ) : j.status === "running" ? (
                              <Badge tone="amber">running</Badge>
                            ) : (
                              <Badge tone="brand">{j.status}</Badge>
                            )}
                            {j.progress && (
                              <span className="ml-2 text-xs text-muted">
                                {j.progress.succeeded}/{j.progress.total}
                              </span>
                            )}
                          </td>
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
          </div>
        )}
      </Container>

      {selected && (
        <OutputLightbox
          job={selected.job}
          modelName={MODEL_BY_SLUG[selected.job.model]?.name ?? selected.job.model}
          initialIndex={selected.index}
          loadRequest={(id) => api.adminGetJobRequest(id)}
          extraStats={[["Account", selected.job.account_id]]}
          onClose={() => setSelected(null)}
        />
      )}
    </div>
  );
}
