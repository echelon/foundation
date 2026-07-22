/**
 * The REAL playground for live (gateway-routable) image models — GPT Image 1 / 1 Mini /
 * 1.5 / 2. Enqueues through the actual API (`POST /v1/models/{slug}/generate`), polls
 * `GET /v1/jobs/{id}`, and renders outputs from CDN URLs as each image lands (partial
 * completion for n > 1). Also builds the request JSON live from the form (for both the
 * model-specific and Any planes) and documents both endpoints.
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ImagePlus, Loader2, Play, RotateCcw, Sparkles, X } from "lucide-react";
import { Badge, Button, Card, Label, Select, Slider, Textarea } from "@/components/ui";
import { CodeBlock } from "@/components/CodeBlock";
import { AccountModal } from "@/components/AccountModal";
import { BatchBadge } from "@/components/BatchBadge";
import { OutputLightbox, sessionRequestLoader, StatusBadge } from "@/components/OutputLightbox";
import { useAuth } from "@/lib/auth";
import {
  api,
  API_BASE_URL,
  ApiError,
  type ApiAccount,
  type ApiCostQuote,
  type ApiJob,
  type ApiJobOutput,
  type ApiJobRequest,
  type ImageGenerationParams,
  type UploadedFile,
} from "@/lib/api";
import { accountDisplayName } from "@/lib/api";
import type { Model } from "@/lib/catalog";
import { cn } from "@/lib/cn";

/* ── Model capability map (mirrors the backend router's builds) ── */

const SIZES = [
  { value: "auto", label: "Auto" },
  { value: "1024x1024", label: "1024 × 1024 · Square" },
  { value: "1536x1024", label: "1536 × 1024 · Landscape" },
  { value: "1024x1536", label: "1024 × 1536 · Portrait" },
] as const;
const QUALITIES = [
  { value: "auto", label: "Auto" },
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
] as const;
const FORMATS = [
  { value: "png", label: "PNG" },
  { value: "jpeg", label: "JPEG" },
  { value: "webp", label: "WebP" },
] as const;

/** gpt-image-2 has no input_fidelity — the router would prune it (and report it). */
function supportsInputFidelity(slug: string): boolean {
  return slug !== "openai_gpt_image_2";
}

type FormState = {
  prompt: string;
  dryRun: boolean;
  n: number;
  size: string;
  quality: string;
  outputFormat: string;
  inputFidelity: string;
  inputs: UploadedFile[];
  mask: UploadedFile | null;
};

const INITIAL_FORM: FormState = {
  prompt: "",
  dryRun: false,
  n: 1,
  size: "auto",
  quality: "auto",
  outputFormat: "png",
  inputFidelity: "",
  inputs: [],
  mask: null,
};

/** Per-model playground state that survives navigating away and back (SPA lifetime).
 * Keyed by model slug: the form as last edited plus the most recent request's job. */
const PLAYGROUND_STATE = new Map<string, { form: FormState; job: ApiJob | null }>();

/** Re-fill the form from a job's originating request (`GET /v1/jobs/{id}/request` — the
 * same wire vocabulary the form produces; image refs come back as CDN URLs, which the
 * API accepts as inputs directly). */
function formFromRequest(r: ApiJobRequest): FormState {
  const asUpload = (url: string): UploadedFile => ({ file_id: url, cdn_url: url, mime: "", bytes: 0, width: 0, height: 0 });
  return {
    prompt: r.prompt,
    dryRun: r.dry_run,
    n: r.n,
    size: r.size ?? "auto",
    quality: r.quality ?? "auto",
    outputFormat: r.output_format ?? "png",
    inputFidelity: r.input_fidelity ?? "",
    inputs: r.image_inputs.map(asUpload),
    mask: r.mask ? asUpload(r.mask) : null,
  };
}

function buildParams(form: FormState, accountId: string): ImageGenerationParams {
  const p: ImageGenerationParams = { account_id: accountId, prompt: form.prompt, n: form.n };
  if (form.inputs.length) p.image_inputs = form.inputs.map((f) => f.file_id);
  if (form.mask) p.mask = form.mask.file_id;
  if (form.size !== "auto") p.size = form.size as ImageGenerationParams["size"];
  if (form.quality !== "auto") p.quality = form.quality as ImageGenerationParams["quality"];
  if (form.outputFormat !== "png") p.output_format = form.outputFormat as ImageGenerationParams["output_format"];
  if (form.inputFidelity && form.inputs.length) p.input_fidelity = form.inputFidelity as ImageGenerationParams["input_fidelity"];
  if (form.dryRun) p.dry_run = true;
  return p;
}

export function LiveImagePlayground({ model }: { model: Model }) {
  const { user } = useAuth();
  const [accounts, setAccounts] = useState<ApiAccount[]>([]);
  const [accountId, setAccountId] = useState<string>("");
  const [form, setForm] = useState<FormState>(() => PLAYGROUND_STATE.get(model.slug)?.form ?? INITIAL_FORM);
  const [uploading, setUploading] = useState<"input" | "mask" | null>(null);
  const [job, setJob] = useState<ApiJob | null>(() => PLAYGROUND_STATE.get(model.slug)?.job ?? null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [quote, setQuote] = useState<ApiCostQuote | null>(null);
  const [showAccountModal, setShowAccountModal] = useState(false);
  const [historyNonce, setHistoryNonce] = useState(0);
  const [lightboxIndex, setLightboxIndex] = useState<number | null>(null);
  const pollTimer = useRef<number | null>(null);
  /** Invalidates in-flight poll ticks when the result is cleared or the model swaps. */
  const pollEpoch = useRef(0);
  const topRef = useRef<HTMLDivElement>(null);
  /** The slug the current form/job state belongs to (the prop can change without remount). */
  const activeSlug = useRef(model.slug);

  const set = <K extends keyof FormState>(k: K, v: FormState[K]) => setForm((f) => ({ ...f, [k]: v }));

  /* Persist the latest form + result so navigating away and back restores them. Writes
     under the ACTIVE slug — on a model switch this runs before the swap effect below. */
  useEffect(() => {
    PLAYGROUND_STATE.set(activeSlug.current, { form, job });
  }, [form, job]);

  /* Model switched without a remount: load the new model's saved state. */
  useEffect(() => {
    if (activeSlug.current === model.slug) return;
    activeSlug.current = model.slug;
    pollEpoch.current += 1;
    if (pollTimer.current) window.clearTimeout(pollTimer.current);
    const cached = PLAYGROUND_STATE.get(model.slug);
    setForm(cached?.form ?? INITIAL_FORM);
    setJob(cached?.job ?? null);
    setRunning(false);
    setError(null);
  }, [model.slug]);

  /* Load the caller's organizations (the job needs an owning account). */
  useEffect(() => {
    if (!user) return;
    let dead = false;
    api
      .listAccounts()
      .then(({ accounts }) => {
        if (dead) return;
        setAccounts(accounts);
        if (accounts.length && !accounts.some((a) => a.account_id === accountId)) {
          setAccountId(accounts[0].account_id);
        }
      })
      .catch(() => {});
    return () => {
      dead = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user]);

  /* Debounced live cost quote for the current form. */
  useEffect(() => {
    if (!accountId || !form.prompt.trim()) {
      setQuote(null);
      return;
    }
    const t = window.setTimeout(() => {
      api
        .costWithModel(model.slug, buildParams(form, accountId))
        .then(setQuote)
        .catch(() => setQuote(null));
    }, 400);
    return () => window.clearTimeout(t);
  }, [form, accountId, model.slug]);

  /* Poll the enqueued job until terminal; outputs render as they arrive. */
  const poll = useCallback(
    (jobId: string, acct: string) => {
      const epoch = pollEpoch.current;
      const tick = async () => {
        try {
          const j = await api.getJob(jobId, acct);
          if (epoch !== pollEpoch.current) return; // result was cleared / model swapped
          setJob(j);
          if (j.status === "queued" || j.status === "running") {
            pollTimer.current = window.setTimeout(tick, 2000);
          } else {
            setRunning(false);
            setHistoryNonce((n) => n + 1);
          }
        } catch {
          if (epoch !== pollEpoch.current) return;
          pollTimer.current = window.setTimeout(tick, 4000);
        }
      };
      void tick();
    },
    [],
  );
  useEffect(() => () => {
    if (pollTimer.current) window.clearTimeout(pollTimer.current);
  }, []);

  /* A restored job that was still in flight when the user navigated away: resume polling
     once the account is known. The guards make re-runs (each poll tick updates `job`) a
     no-op while a poll loop is live or the job is terminal. */
  useEffect(() => {
    if (!accountId || !job || running) return;
    if (job.status !== "queued" && job.status !== "running") return;
    setRunning(true);
    poll(job.id, accountId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [accountId, job, running]);

  const execute = async () => {
    if (!accountId) return;
    setError(null);
    setJob(null);
    setRunning(true);
    try {
      const j = await api.generateWithModel(model.slug, buildParams(form, accountId));
      setJob(j);
      setHistoryNonce((n) => n + 1);
      poll(j.id, accountId);
    } catch (e) {
      setRunning(false);
      setError(e instanceof ApiError ? e.message : "Request failed");
    }
  };

  const run = async () => {
    if (!user) {
      setShowAccountModal(true);
      return;
    }
    await execute();
  };

  const upload = async (kind: "input" | "mask", file: File) => {
    if (!user) {
      setShowAccountModal(true);
      return;
    }
    if (!accountId) return;
    setUploading(kind);
    setError(null);
    try {
      const uploaded = await api.uploadFile(accountId, file);
      if (kind === "input") setForm((f) => ({ ...f, inputs: [...f.inputs, uploaded] }));
      else setForm((f) => ({ ...f, mask: uploaded }));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Upload failed");
    } finally {
      setUploading(null);
    }
  };

  /** Reset the REQUEST side to defaults (the result pane is untouched — that's Clear). */
  const reset = () => {
    setForm(INITIAL_FORM);
    setError(null);
  };

  /** Blank the result pane (stops any in-flight polling; the job itself is unaffected). */
  const clearResult = () => {
    pollEpoch.current += 1;
    if (pollTimer.current) window.clearTimeout(pollTimer.current);
    setJob(null);
    setRunning(false);
    setLightboxIndex(null);
  };

  /** Re-fill the form from a past job's originating request — the request JSON below
   * rebuilds from the form, so it comes back too. */
  const recreate = async (j: ApiJob) => {
    if (!accountId) return;
    try {
      const req = await api.getJobRequest(j.id, accountId);
      setForm(formFromRequest(req));
      setError(null);
      topRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Couldn't load the job's request");
    }
  };

  const isEdit = form.inputs.length > 0;

  return (
    <div className="space-y-6">
      <div ref={topRef} className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.1fr)]">
        {/* ── Controls ── */}
        <Card className="p-5">
          <div className="mb-4 flex items-center justify-between">
            <h3 className="font-display font-semibold">Inputs</h3>
            <div className="flex items-center gap-2">
              <Badge tone="green">Live API</Badge>
              <Badge tone="brand">{isEdit ? "Image edit" : "Text to image"}</Badge>
            </div>
          </div>
          <div className="space-y-4">
            {user && accounts.length > 1 && (
              <div>
                <Label hint="jobs bill to this organization">Organization</Label>
                <Select value={accountId} onChange={(e) => setAccountId(e.target.value)}>
                  {accounts.map((a) => (
                    <option key={a.account_id} value={a.account_id}>
                      {accountDisplayName(a)}
                    </option>
                  ))}
                </Select>
              </div>
            )}
            <div>
              <Label hint="required">Prompt</Label>
              <Textarea
                rows={3}
                placeholder="Describe what you want to generate…"
                value={form.prompt}
                onChange={(e) => set("prompt", e.target.value)}
              />
            </div>

            {/* Image inputs (edit mode) + mask */}
            <div>
              <Label hint="attach to switch to edit mode">Input images</Label>
              <div className="flex flex-wrap items-center gap-2">
                {form.inputs.map((f) => (
                  <div key={f.file_id} className="group relative">
                    <img src={f.cdn_url} alt="" className="h-16 w-16 rounded-lg border border-border object-cover" />
                    <button
                      type="button"
                      onClick={() => setForm((s) => ({ ...s, inputs: s.inputs.filter((x) => x.file_id !== f.file_id) }))}
                      className="absolute -right-1.5 -top-1.5 grid h-5 w-5 place-items-center rounded-full bg-surface2 text-muted opacity-0 shadow transition group-hover:opacity-100"
                    >
                      <X className="h-3 w-3" />
                    </button>
                  </div>
                ))}
                <UploadTile
                  busy={uploading === "input"}
                  label="Add image"
                  onFile={(file) => void upload("input", file)}
                />
              </div>
            </div>
            {isEdit && (
              <div>
                <Label hint="optional — transparent regions are regenerated">Inpainting mask</Label>
                <div className="flex items-center gap-2">
                  {form.mask ? (
                    <div className="group relative">
                      <img src={form.mask.cdn_url} alt="" className="h-16 w-16 rounded-lg border border-border object-cover" />
                      <button
                        type="button"
                        onClick={() => set("mask", null)}
                        className="absolute -right-1.5 -top-1.5 grid h-5 w-5 place-items-center rounded-full bg-surface2 text-muted opacity-0 shadow transition group-hover:opacity-100"
                      >
                        <X className="h-3 w-3" />
                      </button>
                    </div>
                  ) : (
                    <UploadTile busy={uploading === "mask"} label="Add mask" onFile={(file) => void upload("mask", file)} />
                  )}
                </div>
              </div>
            )}

            <div>
              <Label hint={`${form.n} image${form.n > 1 ? "s" : ""} — generated independently, results arrive as each finishes`}>
                Images
              </Label>
              <Slider value={form.n} min={1} max={10} onChange={(v) => set("n", v)} />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div>
                <Label>Size</Label>
                <Select value={form.size} onChange={(e) => set("size", e.target.value)}>
                  {SIZES.map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </Select>
              </div>
              <div>
                <Label>Quality</Label>
                <Select value={form.quality} onChange={(e) => set("quality", e.target.value)}>
                  {QUALITIES.map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </Select>
              </div>
              <div>
                <Label>Output format</Label>
                <Select value={form.outputFormat} onChange={(e) => set("outputFormat", e.target.value)}>
                  {FORMATS.map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </Select>
              </div>
              {isEdit && supportsInputFidelity(model.slug) && (
                <div>
                  <Label hint="how closely edits keep the input">Input fidelity</Label>
                  <Select value={form.inputFidelity} onChange={(e) => set("inputFidelity", e.target.value)}>
                    <option value="">Default</option>
                    <option value="low">Low</option>
                    <option value="high">High</option>
                  </Select>
                </div>
              )}
            </div>
          </div>

          <label className="mt-4 flex items-center justify-between rounded-lg border border-border bg-surface2/40 px-3 py-2.5">
            <span>
              <span className="block text-sm font-medium text-fg">Dry run</span>
              <span className="block text-xs text-muted">
                Validate, price, and log the exact provider payload — nothing is sent, nothing is billed.
              </span>
            </span>
            <button
              type="button"
              onClick={() => set("dryRun", !form.dryRun)}
              className={cn("relative h-6 w-11 shrink-0 rounded-full transition", form.dryRun ? "brand-gradient" : "bg-surface2")}
            >
              <span className={cn("absolute top-0.5 h-5 w-5 rounded-full bg-white transition", form.dryRun ? "left-[1.375rem]" : "left-0.5")} />
            </button>
          </label>

          <div className="mt-4 flex items-center gap-2">
            <Button onClick={run} disabled={running || !form.prompt.trim()} className="flex-1">
              {running ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
              {running ? "Generating…" : form.dryRun ? `Dry run (${form.n})` : `Run (${form.n})`}
            </Button>
            <Button variant="outline" onClick={reset} title="Reset the request to defaults">
              <RotateCcw className="h-4 w-4" />
              Reset
            </Button>
          </div>
          <p className="mt-3 text-xs text-muted">
            {quote
              ? `Estimated ${quote.display} for ${form.n} image${form.n > 1 ? "s" : ""} — ${quote.certainty === "estimated" ? "estimate; provider usage is authoritative" : quote.certainty}.`
              : "Real request — billed to your organization (billing is off during the preview)."}
          </p>
          {quote && quote.param_adjustments.length > 0 && (
            <p className="mt-2 rounded-lg bg-amber-500/10 px-3 py-2 text-xs text-amber-600 dark:text-amber-400">
              {quote.param_adjustments.map((a) => `${a.param}: ${a.detail}`).join(" · ")}
            </p>
          )}
          {error && <p className="mt-2 rounded-lg bg-red-500/10 px-3 py-2 text-xs text-red-500">{error}</p>}
        </Card>

        {/* ── Output ── */}
        <Card className="flex flex-col overflow-hidden">
          <div className="flex items-center justify-between border-b border-border px-5 py-3">
            <h3 className="font-display font-semibold">Output</h3>
            <div className="flex items-center gap-2">
              {job?.progress && (job.status === "queued" || job.status === "running") && (
                <span className="text-xs text-muted">
                  {job.progress.completed}/{job.progress.total} done
                </span>
              )}
              {job && <StatusBadge status={job.status} />}
              {job && (
                <>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => void recreate(job)}
                    title="Re-fill the request form from this result"
                  >
                    <RotateCcw className="h-3.5 w-3.5" />
                    Recreate
                  </Button>
                  <Button variant="ghost" size="sm" onClick={clearResult} title="Blank the result pane">
                    <X className="h-3.5 w-3.5" />
                    Clear
                  </Button>
                </>
              )}
            </div>
          </div>
          <div className="flex-1 p-5">
            {!job && !running && (
              <div className="grid h-full min-h-[16rem] place-items-center text-center text-muted">
                <div>
                  <Sparkles className="mx-auto mb-3 h-8 w-8 text-brand/60" />
                  <p className="text-sm">Set your inputs and hit Run — this playground talks to the real API.</p>
                </div>
              </div>
            )}
            {job && (
              <div className="space-y-4">
                {job.progress && (
                  <div className="h-1.5 w-full overflow-hidden rounded-full bg-surface2">
                    <div
                      className="h-full rounded-full brand-gradient transition-all"
                      style={{ width: `${(job.progress.completed / job.progress.total) * 100}%` }}
                    />
                  </div>
                )}
                <div className={cn("grid gap-3", job.outputs.length > 1 || (job.progress?.total ?? 1) > 1 ? "grid-cols-2 sm:grid-cols-3" : "grid-cols-1")}>
                  {job.outputs.map((o, i) => (
                    <button
                      key={o.index}
                      type="button"
                      onClick={() => setLightboxIndex(i)}
                      className="group block text-left"
                      title="View details"
                    >
                      <img
                        src={o.url}
                        alt={`Output ${o.index + 1}`}
                        className="w-full rounded-xl border border-border object-cover transition group-hover:opacity-90"
                      />
                    </button>
                  ))}
                  {(job.status === "queued" || job.status === "running") &&
                    Array.from({ length: Math.max((job.progress?.total ?? 1) - job.outputs.length, job.status === "queued" && !job.progress ? 1 : 0) }).map((_, i) => (
                      <div key={`pending-${i}`} className="grid aspect-square w-full animate-pulse place-items-center rounded-xl bg-surface2">
                        <Loader2 className="h-5 w-5 animate-spin text-muted" />
                      </div>
                    ))}
                </div>
                {job.dry_run && (
                  <p className="rounded-lg bg-brand/10 px-3 py-2 text-sm text-fg">
                    Dry run complete — nothing was sent to the provider. The exact request payload
                    {(job.progress?.total ?? 1) > 1 ? "s were" : " was"} recorded in the outbound request log.
                  </p>
                )}
                {job.status === "failed" && (
                  <p className="rounded-lg bg-red-500/10 px-3 py-2 text-sm text-red-500">
                    {job.failure_message ?? "Generation failed."}
                  </p>
                )}
                {job.status === "succeeded" && job.progress && job.progress.failed > 0 && (
                  <p className="rounded-lg bg-amber-500/10 px-3 py-2 text-xs text-amber-600 dark:text-amber-400">
                    {job.progress.failed} of {job.progress.total} images failed — you get (and pay for) the ones that
                    succeeded.
                  </p>
                )}
                <div className="grid grid-cols-3 gap-3 text-sm">
                  <Stat label="Job" value={job.id} mono />
                  <Stat label="Status" value={job.status} />
                  <Stat label="Est. cost" value={`$${(job.estimated_micros / 1_000_000).toFixed(4)}`} />
                </div>
                {job.param_adjustments.length > 0 && (
                  <p className="rounded-lg bg-surface2/60 px-3 py-2 text-xs text-muted">
                    Adjusted: {job.param_adjustments.map((a) => `${a.param} (${a.action}) — ${a.detail}`).join("; ")}
                  </p>
                )}
              </div>
            )}
          </div>
        </Card>
      </div>

      <RequestJsonBuilder model={model} form={form} accountId={accountId || "acct_YOUR_ACCOUNT"} />
      <ApiDocs model={model} />
      {user && accountId && <LiveHistory model={model} accountId={accountId} nonce={historyNonce} onRecreate={recreate} />}

      <AccountModal
        open={showAccountModal}
        onClose={() => setShowAccountModal(false)}
        onAuthed={() => void execute()}
        reason={`Create a free account to run ${model.name} in the playground.`}
      />

      {job && lightboxIndex !== null && (
        <OutputLightbox
          job={job}
          modelName={model.name}
          initialIndex={lightboxIndex}
          loadRequest={sessionRequestLoader(accountId)}
          onClose={() => setLightboxIndex(null)}
        />
      )}
    </div>
  );
}

/* ── Upload tile ── */

function UploadTile({ label, busy, onFile }: { label: string; busy: boolean; onFile: (f: File) => void }) {
  const inputRef = useRef<HTMLInputElement>(null);
  return (
    <>
      <button
        type="button"
        onClick={() => inputRef.current?.click()}
        disabled={busy}
        className="grid h-16 w-16 place-items-center rounded-lg border border-dashed border-border text-muted transition hover:border-brand hover:text-brand"
        title={label}
      >
        {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <ImagePlus className="h-4 w-4" />}
      </button>
      <input
        ref={inputRef}
        type="file"
        accept="image/png,image/jpeg,image/webp,image/gif"
        className="hidden"
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (f) onFile(f);
          e.target.value = "";
        }}
      />
    </>
  );
}

/* ── Interactive request-JSON builder ── */

function RequestJsonBuilder({ model, form, accountId }: { model: Model; form: FormState; accountId: string }) {
  const [plane, setPlane] = useState<"model" | "any">("model");
  const params = buildParams(form, accountId);
  const body: Record<string, unknown> = plane === "any" ? { model: model.slug, ...params } : { ...params };
  if (!form.prompt.trim()) body.prompt = "A watercolor lighthouse at dusk";
  const path = plane === "any" ? "/v1/any/image" : `/v1/models/${model.slug}/generate`;
  const json = JSON.stringify(body, null, 2);
  const curl = `curl -X POST ${API_BASE_URL}${path} \\\n  -H "Authorization: Bearer $FOUNDATION_API_KEY" \\\n  -H "Content-Type: application/json" \\\n  -d '${json.replace(/'/g, "\\'")}'`;

  return (
    <Card className="overflow-hidden">
      <div className="flex flex-col gap-3 border-b border-border px-5 py-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h3 className="font-display font-semibold">Request builder</h3>
          <p className="text-xs text-muted">The exact request your current form settings produce — updates live.</p>
        </div>
        <div className="inline-flex rounded-lg border border-border bg-surface p-1">
          {(
            [
              ["model", "Model API"],
              ["any", "Any API"],
            ] as const
          ).map(([key, label]) => (
            <button
              key={key}
              onClick={() => setPlane(key)}
              className={cn(
                "rounded-md px-3 py-1.5 text-xs font-medium transition",
                plane === key ? "brand-gradient text-white shadow-glow" : "text-muted hover:text-fg",
              )}
            >
              {label}
            </button>
          ))}
        </div>
      </div>
      <div className="border-b border-border bg-surface2/40 px-5 py-2 font-mono text-xs text-muted">
        POST {path} <span className="text-muted/70">· also priced (never run) at</span> POST {path}
        {plane === "any" ? "/cost" : "/cost"}
      </div>
      <CodeBlock code={curl} lang="bash" />
    </Card>
  );
}

/* ── Per-model API documentation (both planes) ── */

function ApiDocs({ model }: { model: Model }) {
  const rows: [string, string, string][] = [
    ["account_id", "string · required*", "The owning organization. *Session callers only — API-key callers omit it (the key implies the account)."],
    ["prompt", "string · required", "What to generate. The only universally required param."],
    ["image_inputs", "string[]", "Edit sources: `file_…` ids from POST /v1/files, or public https URLs. Presence switches the job to edit mode."],
    ["mask", "string", "Inpainting mask (`file_…` id or https URL). Transparent regions are regenerated. Requires image_inputs."],
    ["n", "integer · 1–10 · default 1", "Images per request. Each generates independently — poll shows partial completion; you keep whatever succeeds."],
    ["size", '"1024x1024" | "1536x1024" | "1024x1536" | "auto"', "Output dimensions."],
    ["quality", '"low" | "medium" | "high" | "auto"', "Rendering quality tier — drives price."],
    ["output_format", '"png" | "jpeg" | "webp"', "Encoding of the stored outputs."],
    [
      "input_fidelity",
      '"low" | "high"',
      supportsInputFidelity(model.slug)
        ? "How closely edits preserve the input images."
        : "NOT supported by this model — if sent, it is pruned and reported in param_adjustments.",
    ],
  ];
  return (
    <Card className="overflow-hidden">
      <div className="border-b border-border px-5 py-4">
        <h3 className="font-display font-semibold">API reference — {model.name}</h3>
        <p className="mt-1 text-xs text-muted">
          Two equivalent planes: the model-specific endpoint (model in the path) and the Any endpoint (model as a
          param — swap models without changing shape). Both return <code className="rounded bg-surface2 px-1">201</code>{" "}
          with a Job; poll <code className="rounded bg-surface2 px-1">GET /v1/jobs/{"{id}"}</code> (or the cheap{" "}
          <code className="rounded bg-surface2 px-1">/status</code> twin) until terminal. Requests are never held open.
        </p>
        <div className="mt-3 space-y-1 font-mono text-xs">
          <p>
            <span className="mr-2 rounded bg-brand/10 px-1.5 py-0.5 font-semibold text-brand">POST</span>
            /v1/models/{model.slug}/generate <span className="text-muted">· cost twin: …/generate/cost</span>
          </p>
          <p>
            <span className="mr-2 rounded bg-brand/10 px-1.5 py-0.5 font-semibold text-brand">POST</span>
            /v1/any/image <span className="text-muted">{'· body includes "model": "'}{model.slug}{'" · cost twin: /v1/any/image/cost'}</span>
          </p>
          <p>
            <span className="mr-2 rounded bg-surface2 px-1.5 py-0.5 font-semibold text-muted">POST</span>
            /v1/files?account_id=… <span className="text-muted">· raw image bytes → file_… id + CDN URL (edit inputs)</span>
          </p>
        </div>
      </div>
      <div className="scrollbar-thin overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border text-left text-xs text-muted">
              <th className="py-2 pl-5 pr-3 font-medium">Param</th>
              <th className="px-3 py-2 font-medium">Type</th>
              <th className="py-2 pl-3 pr-5 font-medium">Notes</th>
            </tr>
          </thead>
          <tbody>
            {rows.map(([param, type, notes]) => (
              <tr key={param} className="border-b border-border/60 align-top last:border-0">
                <td className="py-2.5 pl-5 pr-3 font-mono text-[13px] text-fg">{param}</td>
                <td className="px-3 py-2.5 font-mono text-xs text-muted">{type}</td>
                <td className="py-2.5 pl-3 pr-5 text-xs text-muted">{notes}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p className="border-t border-border px-5 py-3 text-xs text-muted">
        Responses include <code className="rounded bg-surface2 px-1">estimated_micros</code> (USD micros — GPT Image
        pricing is an estimate; provider usage is authoritative), <code className="rounded bg-surface2 px-1">param_adjustments</code>{" "}
        (anything pruned/coerced to fit the model), and for n&gt;1 a{" "}
        <code className="rounded bg-surface2 px-1">progress</code> block with per-image completion.
      </p>
    </Card>
  );
}

/* ── Real request history ── */

function LiveHistory({
  model,
  accountId,
  nonce,
  onRecreate,
}: {
  model: Model;
  accountId: string;
  nonce: number;
  onRecreate: (j: ApiJob) => void;
}) {
  const [jobs, setJobs] = useState<ApiJob[]>([]);
  const [nextAfter, setNextAfter] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let dead = false;
    setLoading(true);
    api
      .listJobs(accountId, { model: model.slug })
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
  }, [accountId, model.slug, nonce]);

  const loadMore = async () => {
    if (!nextAfter) return;
    setLoading(true);
    try {
      const page = await api.listJobs(accountId, { model: model.slug, after: nextAfter });
      setJobs((j) => [...j, ...page.jobs]);
      setNextAfter(page.next_after);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card className="overflow-hidden">
      <div className="border-b border-border px-5 py-4">
        <h3 className="font-display font-semibold">Request history</h3>
        <p className="text-xs text-muted">Real {model.name} jobs on this organization, newest first.</p>
      </div>
      {jobs.length === 0 && (
        <p className="px-5 py-6 text-sm text-muted">{loading ? "Loading…" : "No jobs yet — your runs will appear here."}</p>
      )}
      {jobs.length > 0 && (
        <div className="scrollbar-thin overflow-x-auto">
          <table className="w-full text-sm">
            <tbody>
              {jobs.map((j) => (
                <tr key={j.id} className="border-b border-border/60 last:border-0 hover:bg-surface2/40">
                  <td className="py-3 pl-5 pr-3">
                    <div className="flex items-center gap-2">
                      <div className="flex -space-x-2">
                        {j.outputs.slice(0, 3).map((o) => (
                          <img key={o.index} src={o.url} alt="" className="h-10 w-10 rounded-lg border border-border object-cover" />
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
                  <td className="px-3 py-3">
                    <StatusBadge status={j.status} />
                    {j.progress && (
                      <span className="ml-2 text-xs text-muted">
                        {j.progress.succeeded}/{j.progress.total}
                      </span>
                    )}
                  </td>
                  <td className="hidden px-3 py-3 text-xs text-muted sm:table-cell">{j.modality.replace(/_/g, " ")}</td>
                  <td className="px-3 py-3 text-right font-medium text-fg">
                    ${(j.estimated_micros / 1_000_000).toFixed(4)}
                  </td>
                  <td className="py-3 pl-3 pr-5 text-right">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => void onRecreate(j)}
                      title="Re-fill the request form from this job"
                    >
                      <RotateCcw className="h-3.5 w-3.5" />
                      Recreate
                    </Button>
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
  );
}


function Stat({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="rounded-lg bg-surface2/60 px-3 py-2">
      <div className="text-xs text-muted">{label}</div>
      <div className={cn("truncate font-medium text-fg", mono && "font-mono text-[13px]")}>{value}</div>
    </div>
  );
}
