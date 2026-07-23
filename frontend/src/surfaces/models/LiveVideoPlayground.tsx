/**
 * The REAL playground for live (gateway-routable) VIDEO models — the Veo 3.1 family.
 * Enqueues through the actual API (`POST /v1/models/{slug}/generate`), polls
 * `GET /v1/jobs/{id}` while the async provider operation renders (minutes, not
 * seconds), and plays outputs from CDN URLs as each video lands (partial completion
 * for n > 1). Mirrors LiveImagePlayground: live request-JSON builder for both API
 * planes plus per-model endpoint docs.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { Loader2, Play, RotateCcw, Sparkles, X } from "lucide-react";
import { Badge, Button, Card, Input, Label, Select, Slider, Textarea } from "@/components/ui";
import { CodeBlock } from "@/components/CodeBlock";
import { AccountModal } from "@/components/AccountModal";
import { OutputLightbox, sessionRequestLoader, StatusBadge } from "@/components/OutputLightbox";
import { useAuth } from "@/lib/auth";
import {
  api,
  API_BASE_URL,
  ApiError,
  accountDisplayName,
  type ApiAccount,
  type ApiCostQuote,
  type ApiJob,
  type ApiJobRequest,
  type UploadedFile,
  type VideoGenerationParams,
} from "@/lib/api";
import type { Model } from "@/lib/catalog";
import { cn } from "@/lib/cn";
import { LiveHistory, Stat, UploadTile } from "./LiveImagePlayground";

/* ── Model capability map (mirrors the backend router's builds) ── */

const DURATIONS = [
  { value: "", label: "Default (8 s)" },
  { value: "4", label: "4 seconds" },
  { value: "6", label: "6 seconds" },
  { value: "8", label: "8 seconds" },
] as const;
const ASPECTS = [
  { value: "", label: "Default (16:9)" },
  { value: "16:9", label: "16:9 · Landscape" },
  { value: "9:16", label: "9:16 · Portrait" },
] as const;

/** Omni rides the Interactions API: no resolution/duration knobs (output length is
 * prompt-driven, ~720p) and no discrete negative prompt (folded into the prompt). */
function isOmni(slug: string): boolean {
  return slug === "google_omni_flash";
}

/** Lite has no 4K — the router would coerce it to 1080p (and report it). */
function resolutionOptions(slug: string): { value: string; label: string }[] {
  const base = [
    { value: "", label: "Default (720p)" },
    { value: "720p", label: "720p" },
    { value: "1080p", label: "1080p · forces 8 s" },
  ];
  if (slug !== "google_veo_3_1_lite") base.push({ value: "4k", label: "4K · forces 8 s" });
  return base;
}

type FormState = {
  prompt: string;
  negativePrompt: string;
  dryRun: boolean;
  n: number;
  resolution: string;
  duration: string;
  aspectRatio: string;
  input: UploadedFile | null;
};

const INITIAL_FORM: FormState = {
  prompt: "",
  negativePrompt: "",
  dryRun: false,
  n: 1,
  resolution: "",
  duration: "",
  aspectRatio: "",
  input: null,
};

/** Per-model playground state that survives navigating away and back (SPA lifetime). */
const PLAYGROUND_STATE = new Map<string, { form: FormState; job: ApiJob | null }>();

/** Re-fill the form from a job's originating request (video jobs report the video
 * param set; the i2v ref comes back as a CDN URL, which `image_input` accepts). */
function formFromRequest(r: ApiJobRequest): FormState {
  const asUpload = (url: string): UploadedFile => ({ file_id: url, cdn_url: url, mime: "", bytes: 0, width: 0, height: 0 });
  return {
    prompt: r.prompt,
    negativePrompt: r.negative_prompt ?? "",
    dryRun: r.dry_run,
    n: r.n,
    resolution: r.resolution ?? "",
    duration: r.duration_seconds ? String(r.duration_seconds) : "",
    aspectRatio: r.aspect_ratio ?? "",
    input: r.image_input ? asUpload(r.image_input) : null,
  };
}

function buildParams(form: FormState, accountId: string): VideoGenerationParams {
  const p: VideoGenerationParams = { account_id: accountId, prompt: form.prompt, n: form.n };
  if (form.negativePrompt.trim()) p.negative_prompt = form.negativePrompt.trim();
  if (form.input) p.image_input = form.input.file_id;
  if (form.resolution) p.resolution = form.resolution as VideoGenerationParams["resolution"];
  if (form.duration) p.duration_seconds = Number(form.duration);
  if (form.aspectRatio) p.aspect_ratio = form.aspectRatio as VideoGenerationParams["aspect_ratio"];
  if (form.dryRun) p.dry_run = true;
  return p;
}

/** Billed seconds for the quote hint: explicit choice, or the coerced/server default. */
function billedSeconds(form: FormState): number {
  if (form.resolution === "1080p" || form.resolution === "4k") return 8;
  return form.duration ? Number(form.duration) : 8;
}

export function LiveVideoPlayground({ model }: { model: Model }) {
  const { user } = useAuth();
  const [accounts, setAccounts] = useState<ApiAccount[]>([]);
  const [accountId, setAccountId] = useState<string>("");
  const [form, setForm] = useState<FormState>(() => PLAYGROUND_STATE.get(model.slug)?.form ?? INITIAL_FORM);
  const [uploading, setUploading] = useState(false);
  const [job, setJob] = useState<ApiJob | null>(() => PLAYGROUND_STATE.get(model.slug)?.job ?? null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [quote, setQuote] = useState<ApiCostQuote | null>(null);
  const [showAccountModal, setShowAccountModal] = useState(false);
  const [historyNonce, setHistoryNonce] = useState(0);
  const [lightboxIndex, setLightboxIndex] = useState<number | null>(null);
  const pollTimer = useRef<number | null>(null);
  const pollEpoch = useRef(0);
  const topRef = useRef<HTMLDivElement>(null);
  const activeSlug = useRef(model.slug);

  const set = <K extends keyof FormState>(k: K, v: FormState[K]) => setForm((f) => ({ ...f, [k]: v }));

  useEffect(() => {
    PLAYGROUND_STATE.set(activeSlug.current, { form, job });
  }, [form, job]);

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

  /* Debounced live cost quote — Veo prices exactly (per second, blocked ≠ billed). */
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

  /* Poll the enqueued job until terminal — Veo renders take minutes; poll gently. */
  const poll = useCallback((jobId: string, acct: string) => {
    const epoch = pollEpoch.current;
    const tick = async () => {
      try {
        const j = await api.getJob(jobId, acct);
        if (epoch !== pollEpoch.current) return;
        setJob(j);
        if (j.status === "queued" || j.status === "running") {
          pollTimer.current = window.setTimeout(tick, 5000);
        } else {
          setRunning(false);
          setHistoryNonce((n) => n + 1);
        }
      } catch {
        if (epoch !== pollEpoch.current) return;
        pollTimer.current = window.setTimeout(tick, 8000);
      }
    };
    void tick();
  }, []);
  useEffect(() => () => {
    if (pollTimer.current) window.clearTimeout(pollTimer.current);
  }, []);

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

  const upload = async (file: File) => {
    if (!user) {
      setShowAccountModal(true);
      return;
    }
    if (!accountId) return;
    setUploading(true);
    setError(null);
    try {
      const uploaded = await api.uploadFile(accountId, file);
      setForm((f) => ({ ...f, input: uploaded }));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Upload failed");
    } finally {
      setUploading(false);
    }
  };

  const reset = () => {
    setForm(INITIAL_FORM);
    setError(null);
  };

  const clearResult = () => {
    pollEpoch.current += 1;
    if (pollTimer.current) window.clearTimeout(pollTimer.current);
    setJob(null);
    setRunning(false);
    setLightboxIndex(null);
  };

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

  const isI2v = form.input !== null;
  const needs8s = form.resolution === "1080p" || form.resolution === "4k";

  return (
    <div className="space-y-6">
      <div ref={topRef} className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.1fr)]">
        {/* ── Controls ── */}
        <Card className="p-5">
          <div className="mb-4 flex items-center justify-between">
            <h3 className="font-display font-semibold">Inputs</h3>
            <div className="flex items-center gap-2">
              <Badge tone="green">Live API</Badge>
              <Badge tone="brand">{isI2v ? "Image to video" : "Text to video"}</Badge>
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
              <Label hint="required — dialogue in quotes and SFX descriptions drive the audio">Prompt</Label>
              <Textarea
                rows={3}
                placeholder="A candle flame flickers in a dark room, slow dolly-in, soft wind sound…"
                value={form.prompt}
                onChange={(e) => set("prompt", e.target.value)}
              />
            </div>
            <div>
              <Label hint={isOmni(model.slug) ? "no discrete param — folded into the prompt as avoidance guidance" : "what NOT to generate"}>Negative prompt</Label>
              <Input
                placeholder="e.g. text overlays, low quality, jump cuts"
                value={form.negativePrompt}
                onChange={(e) => set("negativePrompt", e.target.value)}
              />
            </div>

            {/* i2v conditioning frame */}
            <div>
              <Label hint="attach to switch to image-to-video">Starting image</Label>
              <div className="flex items-center gap-2">
                {form.input ? (
                  <div className="group relative">
                    <img src={form.input.cdn_url} alt="" className="h-16 w-16 rounded-lg border border-border object-cover" />
                    <button
                      type="button"
                      onClick={() => set("input", null)}
                      className="absolute -right-1.5 -top-1.5 grid h-5 w-5 place-items-center rounded-full bg-surface2 text-muted opacity-0 shadow transition group-hover:opacity-100"
                    >
                      <X className="h-3 w-3" />
                    </button>
                  </div>
                ) : (
                  <UploadTile busy={uploading} label="Add image" onFile={(file) => void upload(file)} />
                )}
              </div>
            </div>

            <div>
              <Label hint={`${form.n} video${form.n > 1 ? "s" : ""} — one provider operation each, results arrive as each finishes`}>
                Videos
              </Label>
              <Slider value={form.n} min={1} max={10} onChange={(v) => set("n", v)} />
            </div>
            <div className="grid grid-cols-2 gap-3">
              {!isOmni(model.slug) && (
                <div>
                  <Label>Resolution</Label>
                  <Select value={form.resolution} onChange={(e) => set("resolution", e.target.value)}>
                    {resolutionOptions(model.slug).map((o) => (
                      <option key={o.value} value={o.value}>
                        {o.label}
                      </option>
                    ))}
                  </Select>
                </div>
              )}
              {!isOmni(model.slug) && (
                <div>
                  <Label hint={needs8s ? "1080p/4K require 8 s" : undefined}>Duration</Label>
                  <Select value={form.duration} onChange={(e) => set("duration", e.target.value)}>
                    {DURATIONS.map((o) => (
                      <option key={o.value} value={o.value}>
                        {o.label}
                      </option>
                    ))}
                  </Select>
                </div>
              )}
              <div>
                <Label>Aspect ratio</Label>
                <Select value={form.aspectRatio} onChange={(e) => set("aspectRatio", e.target.value)}>
                  {ASPECTS.map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </Select>
              </div>
            </div>
            {isOmni(model.slug) && (
              <p className="rounded-lg bg-surface2/60 px-3 py-2 text-xs text-muted">
                Omni has no resolution or duration knobs — output length is prompt-driven (~720p,
                token-billed ≈ $0.10/s), so the quote is an estimate and provider usage is authoritative.
              </p>
            )}
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
              {running ? "Rendering…" : form.dryRun ? `Dry run (${form.n})` : `Run (${form.n})`}
            </Button>
            <Button variant="outline" onClick={reset} title="Reset the request to defaults">
              <RotateCcw className="h-4 w-4" />
              Reset
            </Button>
          </div>
          <p className="mt-3 text-xs text-muted">
            {quote
              ? `${quote.certainty === "exact" ? "Exactly" : "Estimated"} ${quote.display} — ${form.n} × ~${billedSeconds(form)} s at ${model.pricing[0] ? `$${model.pricing[0].usd.toFixed(2)}/s` : "the per-second rate"}. Renders take a few minutes.`
              : "Real request — billed per video second to your organization (billing is off during the preview)."}
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
                  <p className="text-sm">
                    Set your inputs and hit Run — Veo renders asynchronously; results land here in a few minutes.
                  </p>
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
                <div className={cn("grid gap-3", job.outputs.length > 1 || (job.progress?.total ?? 1) > 1 ? "grid-cols-2" : "grid-cols-1")}>
                  {job.outputs.map((o, i) => (
                    <button
                      key={o.index}
                      type="button"
                      onClick={() => setLightboxIndex(i)}
                      className="group block text-left"
                      title="View details"
                    >
                      <video
                        src={o.url}
                        muted
                        loop
                        playsInline
                        autoPlay
                        className="w-full rounded-xl border border-border object-cover transition group-hover:opacity-90"
                      />
                    </button>
                  ))}
                  {(job.status === "queued" || job.status === "running") &&
                    Array.from({ length: Math.max((job.progress?.total ?? 1) - job.outputs.length, job.status === "queued" && !job.progress ? 1 : 0) }).map((_, i) => (
                      <div key={`pending-${i}`} className="grid aspect-video w-full animate-pulse place-items-center rounded-xl bg-surface2">
                        <Loader2 className="h-5 w-5 animate-spin text-muted" />
                      </div>
                    ))}
                </div>
                {(job.status === "queued" || job.status === "running") && !job.dry_run && (
                  <p className="rounded-lg bg-surface2/60 px-3 py-2 text-xs text-muted">
                    Veo is rendering at the provider — typically 1–4 minutes per clip. Leave this open or come back;
                    the job keeps running either way.
                  </p>
                )}
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
                    {job.progress.failed} of {job.progress.total} videos failed — you get (and pay for) the ones that
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

/* ── Interactive request-JSON builder ── */

function RequestJsonBuilder({ model, form, accountId }: { model: Model; form: FormState; accountId: string }) {
  const [plane, setPlane] = useState<"model" | "any">("model");
  const params = buildParams(form, accountId);
  const body: Record<string, unknown> = plane === "any" ? { model: model.slug, ...params } : { ...params };
  if (!form.prompt.trim()) body.prompt = "A candle flame flickers in a dark room, slow dolly-in";
  const path = plane === "any" ? "/v1/any/video" : `/v1/models/${model.slug}/generate`;
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
        POST {path} <span className="text-muted/70">· also priced (never run) at</span> POST {path}/cost
      </div>
      <CodeBlock code={curl} lang="bash" />
    </Card>
  );
}

/* ── Per-model API documentation (both planes) ── */

function ApiDocs({ model }: { model: Model }) {
  const omni = isOmni(model.slug);
  const supports4k = model.slug !== "google_veo_3_1_lite";
  const rows: [string, string, string][] = [
    ["account_id", "string · required*", "The owning organization. *Session callers only — API-key callers omit it (the key implies the account)."],
    ["prompt", "string · required", "What to generate. Dialogue in quotes and SFX descriptions drive the generated audio."],
    [
      "negative_prompt",
      "string",
      omni
        ? "Omni has no discrete negative prompt — it is folded into the prompt as avoidance guidance (reported in param_adjustments)."
        : "What NOT to generate.",
    ],
    ["image_input", "string", "Image-to-video conditioning frame: a `file_…` id from POST /v1/files, or a public https URL."],
    ["n", "integer · 1–10 · default 1", "Videos per request. One provider operation each — poll shows partial completion; you keep whatever succeeds."],
    [
      "resolution",
      omni ? "—" : supports4k ? '"720p" | "1080p" | "4k"' : '"720p" | "1080p"',
      omni
        ? "NOT supported — Omni outputs ~720p; if sent, it is pruned and reported in param_adjustments."
        : supports4k
          ? "Output resolution (default 720p). 1080p/4K force an 8-second duration."
          : "Output resolution (default 720p). 4K is NOT supported on Lite — it is coerced to 1080p and reported. 1080p forces 8 s.",
    ],
    [
      "duration_seconds",
      omni ? "—" : "4 | 6 | 8 · default 8",
      omni
        ? "NOT sent — Omni's output length is prompt-driven; the value only refines the cost estimate (pruned and reported)."
        : "Clip length. Coerced to 8 (and reported) when the resolution demands it.",
    ],
    ["aspect_ratio", '"16:9" | "9:16"', "Landscape (default) or portrait."],
    ["dry_run", "boolean", "Validate, price, and log the exact provider payload — never sent, never billed."],
  ];
  return (
    <Card className="overflow-hidden">
      <div className="border-b border-border px-5 py-4">
        <h3 className="font-display font-semibold">API reference — {model.name}</h3>
        <p className="mt-1 text-xs text-muted">
          Two equivalent planes: the model-specific endpoint (model in the path) and the Any endpoint (model as a
          param — swap models without changing shape). Both return <code className="rounded bg-surface2 px-1">201</code>{" "}
          with a Job. Veo runs ASYNC at the provider: poll{" "}
          <code className="rounded bg-surface2 px-1">GET /v1/jobs/{"{id}"}</code> (or the cheap{" "}
          <code className="rounded bg-surface2 px-1">/status</code> twin) until terminal — renders typically take 1–4
          minutes per clip. {model.slug === "google_omni_flash" ? "Pricing is an ESTIMATE (token-billed; output length is prompt-driven — provider usage is authoritative)." : "Pricing is EXACT: n × seconds × the per-second rate."}
        </p>
        <div className="mt-3 space-y-1 font-mono text-xs">
          <p>
            <span className="mr-2 rounded bg-brand/10 px-1.5 py-0.5 font-semibold text-brand">POST</span>
            /v1/models/{model.slug}/generate <span className="text-muted">· cost twin: …/generate/cost</span>
          </p>
          <p>
            <span className="mr-2 rounded bg-brand/10 px-1.5 py-0.5 font-semibold text-brand">POST</span>
            /v1/any/video <span className="text-muted">{'· body includes "model": "'}{model.slug}{'" · cost twin: /v1/any/video/cost'}</span>
          </p>
          <p>
            <span className="mr-2 rounded bg-surface2 px-1.5 py-0.5 font-semibold text-muted">POST</span>
            /v1/files?account_id=… <span className="text-muted">· raw image bytes → file_… id + CDN URL (the i2v frame)</span>
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
        Responses include <code className="rounded bg-surface2 px-1">estimated_micros</code> (USD micros — Veo pricing
        is exact per the request params), <code className="rounded bg-surface2 px-1">param_adjustments</code> (anything
        pruned/coerced to fit the model), and for n&gt;1 a{" "}
        <code className="rounded bg-surface2 px-1">progress</code> block with per-video completion.
      </p>
    </Card>
  );
}
