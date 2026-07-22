/**
 * The result lightbox/detail modal for a generation job — used by the model playgrounds
 * and the admin console. Shows one output at a time with prev/next navigation inside the
 * batch (buttons or ←/→), the originating request (prompt + params, fetched from the
 * typed `/request` endpoint via `loadRequest`), and job metadata. Tolerates jobs with no
 * outputs (failed/queued) as a metadata-only view. Closes on ✕, backdrop click, or Esc.
 */

import { useEffect, useRef, useState } from "react";
import { Check, ChevronLeft, ChevronRight, Copy, ExternalLink, ImagePlus, X } from "lucide-react";
import { Badge, Button, Modal } from "@/components/ui";
import { api, type ApiJob, type ApiJobRequest } from "@/lib/api";
import { cn } from "@/lib/cn";

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}

/** Clipboard image support is PNG-only in practice — transcode other formats via canvas. */
async function toPngBlob(blob: Blob): Promise<Blob> {
  if (blob.type === "image/png") return blob;
  const bitmap = await createImageBitmap(blob);
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  canvas.getContext("2d")!.drawImage(bitmap, 0, 0);
  return await new Promise((resolve, reject) =>
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("png encode failed"))), "image/png"),
  );
}

export function StatusBadge({ status }: { status: ApiJob["status"] }) {
  if (status === "succeeded") return <Badge tone="green">succeeded</Badge>;
  if (status === "failed") return <Badge tone="red">failed</Badge>;
  if (status === "running") return <Badge tone="amber">running</Badge>;
  if (status === "queued") return <Badge tone="brand">queued</Badge>;
  return <Badge>{status}</Badge>;
}

export function OutputLightbox({
  job,
  modelName,
  initialIndex = 0,
  loadRequest,
  extraStats = [],
  onClose,
}: {
  job: ApiJob;
  modelName: string;
  /** Position in `job.outputs` to open on. */
  initialIndex?: number;
  /** How to fetch the originating request — differs by surface (session vs admin). */
  loadRequest: (jobId: string) => Promise<ApiJobRequest>;
  /** Surface-specific metadata rows (e.g. the admin view adds the account). */
  extraStats?: [string, string][];
  onClose: () => void;
}) {
  const outputs = job.outputs;
  const [idx, setIdx] = useState(() => Math.min(Math.max(initialIndex, 0), Math.max(outputs.length - 1, 0)));
  const [copied, setCopied] = useState<"url" | "image" | null>(null);
  const [copyError, setCopyError] = useState<string | null>(null);
  const [req, setReq] = useState<ApiJobRequest | null>(null);
  const flashTimer = useRef<number | null>(null);

  const output = outputs.length > 0 ? outputs[Math.min(idx, outputs.length - 1)] : null;
  const canPrev = idx > 0;
  const canNext = idx < outputs.length - 1;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowLeft") setIdx((i) => Math.max(i - 1, 0));
      if (e.key === "ArrowRight") setIdx((i) => Math.min(i + 1, Math.max(outputs.length - 1, 0)));
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      if (flashTimer.current) window.clearTimeout(flashTimer.current);
    };
  }, [onClose, outputs.length]);

  /* The originating request (prompt, params) — a separate typed endpoint, not part of
     the Job resource. */
  useEffect(() => {
    let dead = false;
    loadRequest(job.id)
      .then((r) => {
        if (!dead) setReq(r);
      })
      .catch(() => {});
    return () => {
      dead = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [job.id]);

  const flash = (kind: "url" | "image") => {
    setCopied(kind);
    setCopyError(null);
    if (flashTimer.current) window.clearTimeout(flashTimer.current);
    flashTimer.current = window.setTimeout(() => setCopied(null), 1500);
  };

  const copyUrl = async () => {
    if (!output) return;
    try {
      await navigator.clipboard.writeText(output.url);
      flash("url");
    } catch {
      setCopyError("Couldn't copy the URL — clipboard access was blocked.");
    }
  };

  const copyImage = async () => {
    if (!output) return;
    try {
      const res = await fetch(output.url);
      if (!res.ok) throw new Error(String(res.status));
      const png = await toPngBlob(await res.blob());
      await navigator.clipboard.write([new ClipboardItem({ "image/png": png })]);
      flash("image");
    } catch {
      setCopyError("Couldn't copy the image — your browser may not allow it. Copy the URL instead.");
    }
  };

  const stats: [string, string][] = [
    ["Job", job.id],
    ["Model", modelName],
    ["Status", job.status],
    ...extraStats,
    ["Source", job.source === "api_key" ? "API key" : job.source],
    ["Plane", job.api_shape === "any" ? "any" : "model-specific"],
    ...(output
      ? ([
          ["Output", `${idx + 1} of ${outputs.length}`],
          ["Format", output.mime],
          ["File size", formatBytes(output.bytes)],
        ] as [string, string][])
      : []),
    ["Size", req ? (req.size ?? "auto") : "…"],
    ["Quality", req ? (req.quality ?? "auto") : "…"],
    ["Est. cost", `$${(job.estimated_micros / 1_000_000).toFixed(4)}`],
    ["Created", new Date(job.created_at).toLocaleString()],
  ];

  return (
    <Modal open onClose={onClose} className="max-w-4xl p-0">
      <div className="flex items-center justify-between border-b border-border px-5 py-3">
        <div className="flex items-center gap-2">
          <h3 className="font-display font-semibold">
            {output ? `Output ${idx + 1} of ${outputs.length}` : "Job details"}
          </h3>
          <StatusBadge status={job.status} />
        </div>
        <div className="flex items-center gap-1">
          {outputs.length > 1 && (
            <>
              <button
                onClick={() => setIdx((i) => Math.max(i - 1, 0))}
                disabled={!canPrev}
                title="Previous result (←)"
                className={cn(
                  "rounded-lg p-2 transition",
                  canPrev ? "text-muted hover:bg-surface2 hover:text-fg" : "cursor-default text-muted/30",
                )}
              >
                <ChevronLeft className="h-4 w-4" />
              </button>
              <button
                onClick={() => setIdx((i) => Math.min(i + 1, outputs.length - 1))}
                disabled={!canNext}
                title="Next result (→)"
                className={cn(
                  "rounded-lg p-2 transition",
                  canNext ? "text-muted hover:bg-surface2 hover:text-fg" : "cursor-default text-muted/30",
                )}
              >
                <ChevronRight className="h-4 w-4" />
              </button>
            </>
          )}
          <button
            onClick={onClose}
            title="Close"
            className="rounded-lg p-2 text-muted transition hover:bg-surface2 hover:text-fg"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      </div>

      <div className="grid gap-5 p-5 lg:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
        <div className="grid place-items-center rounded-xl bg-surface2/40 p-2">
          {output ? (
            <img
              src={output.url}
              alt={`Output ${idx + 1}`}
              className="max-h-[65vh] w-auto max-w-full rounded-lg object-contain"
            />
          ) : (
            <div className="grid min-h-[16rem] place-items-center px-6 text-center text-sm text-muted">
              {job.status === "failed"
                ? (job.failure_message ?? "Generation failed — no outputs.")
                : job.dry_run
                  ? "Dry run — nothing was sent to the provider, so there are no outputs."
                  : "No outputs (yet)."}
            </div>
          )}
        </div>

        <div className="min-w-0 space-y-4">
          {output && (
            <div className="flex flex-wrap gap-2">
              <Button variant="outline" size="sm" onClick={() => void copyUrl()}>
                {copied === "url" ? <Check className="h-3.5 w-3.5 text-emerald-500" /> : <Copy className="h-3.5 w-3.5" />}
                {copied === "url" ? "Copied" : "Copy URL"}
              </Button>
              <Button variant="outline" size="sm" onClick={() => void copyImage()}>
                {copied === "image" ? (
                  <Check className="h-3.5 w-3.5 text-emerald-500" />
                ) : (
                  <ImagePlus className="h-3.5 w-3.5" />
                )}
                {copied === "image" ? "Copied" : "Copy image"}
              </Button>
              <Button variant="outline" size="sm" onClick={() => window.open(output.url, "_blank", "noopener")}>
                <ExternalLink className="h-3.5 w-3.5" />
                Open in new tab
              </Button>
            </div>
          )}
          {copyError && <p className="rounded-lg bg-red-500/10 px-3 py-2 text-xs text-red-500">{copyError}</p>}

          <div>
            <div className="text-xs text-muted">Prompt</div>
            <p className="mt-1 max-h-32 overflow-y-auto whitespace-pre-wrap rounded-lg bg-surface2/60 px-3 py-2 text-sm text-fg">
              {req ? req.prompt : "Loading…"}
            </p>
          </div>

          <div className="grid grid-cols-2 gap-2 text-sm">
            {stats.map(([label, value]) => (
              <div key={label} className="rounded-lg bg-surface2/60 px-3 py-2">
                <div className="text-xs text-muted">{label}</div>
                <div className="truncate font-medium text-fg" title={value}>
                  {value}
                </div>
              </div>
            ))}
          </div>

          {req && req.image_inputs.length > 0 && (
            <div>
              <div className="text-xs text-muted">Edit inputs</div>
              <div className="mt-1 flex flex-wrap gap-2">
                {req.image_inputs.map((u) => (
                  <img key={u} src={u} alt="" className="h-12 w-12 rounded-lg border border-border object-cover" />
                ))}
              </div>
            </div>
          )}
          {job.param_adjustments.length > 0 && (
            <p className="rounded-lg bg-surface2/60 px-3 py-2 text-xs text-muted">
              Adjusted: {job.param_adjustments.map((a) => `${a.param} (${a.action}) — ${a.detail}`).join("; ")}
            </p>
          )}
        </div>
      </div>
    </Modal>
  );
}

/** The default session-scoped request loader (playgrounds, account dashboard). */
export const sessionRequestLoader = (accountId: string) => (jobId: string) => api.getJobRequest(jobId, accountId);
