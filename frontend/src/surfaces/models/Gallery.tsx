import { useMemo, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { Copy, ExternalLink, Play, Wand2, X } from "lucide-react";
import { Badge, Button } from "@/components/ui";
import { cn } from "@/lib/cn";
import { examplesForModel, techniqueLabel, techniquesForModel, type Example } from "@/lib/gallery";

export type RemixValues = Record<string, unknown>;

/** The Prompt Gallery: a filterable grid of curated examples for one model. */
export function Gallery({ slug, onRemix }: { slug: string; onRemix?: (v: RemixValues) => void }) {
  const all = useMemo(() => examplesForModel(slug), [slug]);
  const techniques = useMemo(() => techniquesForModel(slug), [slug]);
  const [active, setActive] = useState<string | null>(null);
  const [open, setOpen] = useState<Example | null>(null);

  const rows = active ? all.filter((e) => e.techniques.includes(active)) : all;

  if (!all.length) {
    return (
      <div className="rounded-2xl border border-dashed border-border py-24 text-center">
        <p className="font-display text-lg font-semibold">No examples yet</p>
        <p className="mx-auto mt-2 max-w-sm text-sm text-muted">Curated prompt examples for this model are on the way.</p>
      </div>
    );
  }

  return (
    <div>
      {/* intro */}
      <div className="mb-5 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <p className="max-w-2xl text-sm text-muted">
          A prompt gallery — {all.length} curated examples showing what {slug} can do and{" "}
          <em>how each effect is achieved</em>. Filter by technique, then remix any example straight into the
          playground.
        </p>
      </div>

      {/* technique filter */}
      <div className="mb-6 flex flex-wrap gap-1.5">
        <button
          onClick={() => setActive(null)}
          className={cn(
            "rounded-lg px-3 py-1.5 text-sm font-medium transition",
            active === null ? "brand-gradient text-white shadow-glow" : "bg-surface2 text-muted hover:text-fg",
          )}
        >
          All
        </button>
        {techniques.map((t) => (
          <button
            key={t}
            onClick={() => setActive(t)}
            className={cn(
              "rounded-lg px-3 py-1.5 text-sm font-medium transition",
              active === t ? "brand-gradient text-white shadow-glow" : "bg-surface2 text-muted hover:text-fg",
            )}
          >
            {techniqueLabel(t)}
          </button>
        ))}
      </div>

      {/* masonry grid */}
      <div className="gap-4 [column-fill:_balance] sm:columns-2 lg:columns-3">
        {rows.map((e) => (
          <ExampleCard key={e.id} example={e} onOpen={() => setOpen(e)} />
        ))}
      </div>

      {open && <ExampleModal example={open} onClose={() => setOpen(null)} onRemix={onRemix} />}
    </div>
  );
}

export function ExampleCard({ example, onOpen }: { example: Example; onOpen: () => void }) {
  return (
    <button onClick={onOpen} className="group mb-4 block w-full break-inside-avoid overflow-hidden rounded-xl border border-border bg-surface text-left transition hover:border-brand/50 hover:shadow-glow">
      <div className="relative">
        <Media example={example} />
        <div className="pointer-events-none absolute inset-0 bg-gradient-to-t from-black/70 via-transparent to-transparent opacity-0 transition group-hover:opacity-100" />
        <div className="pointer-events-none absolute inset-x-0 bottom-0 p-3 opacity-0 transition group-hover:opacity-100">
          <p className="line-clamp-2 text-xs text-white/90">{example.prompt}</p>
        </div>
      </div>
      <div className="p-3">
        <div className="flex items-center justify-between gap-2">
          <h4 className="truncate text-sm font-medium text-fg">{example.title}</h4>
          {example.isVideo && <Badge tone="brand">video</Badge>}
        </div>
        <div className="mt-2 flex flex-wrap gap-1">
          {example.techniques.slice(0, 3).map((t) => (
            <span key={t} className="rounded-full bg-surface2 px-2 py-0.5 text-[11px] text-muted">
              {techniqueLabel(t)}
            </span>
          ))}
        </div>
      </div>
    </button>
  );
}

/** Image or hover-to-play looping video, aspect-aware. */
function Media({ example, large }: { example: Example; large?: boolean }) {
  const ref = useRef<HTMLVideoElement>(null);
  const ratio = aspectToPadding(example.aspect);

  if (example.isVideo) {
    return (
      <div
        className="relative w-full overflow-hidden bg-surface2"
        style={{ paddingBottom: large ? undefined : ratio, aspectRatio: large ? example.aspect.replace(":", "/") : undefined }}
        onMouseEnter={() => ref.current?.play().catch(() => {})}
        onMouseLeave={() => {
          if (ref.current) {
            ref.current.pause();
            ref.current.currentTime = 0;
          }
        }}
      >
        <video
          ref={ref}
          src={example.assetUrl}
          poster={example.posterUrl}
          muted
          loop
          playsInline
          preload="none"
          className={cn("h-full w-full object-cover", large ? "relative" : "absolute inset-0")}
        />
        {!large && (
          <span className="pointer-events-none absolute left-2 top-2 grid h-7 w-7 place-items-center rounded-full bg-black/50 text-white">
            <Play className="h-3.5 w-3.5" />
          </span>
        )}
      </div>
    );
  }
  return (
    <img
      src={example.assetUrl}
      alt={example.title}
      loading="lazy"
      className="w-full object-cover"
      style={{ aspectRatio: example.aspect.replace(":", "/") }}
    />
  );
}

function ExampleModal({ example, onClose, onRemix }: { example: Example; onClose: () => void; onRemix?: (v: RemixValues) => void }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm" onClick={onClose}>
      <div className="scrollbar-thin max-h-[90vh] w-full max-w-4xl overflow-y-auto rounded-2xl border border-border bg-bg shadow-glow" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <h3 className="font-display font-semibold">{example.title}</h3>
          <button onClick={onClose} className="grid h-8 w-8 place-items-center rounded-lg text-muted hover:bg-surface2 hover:text-fg">
            <X className="h-4 w-4" />
          </button>
        </div>
        <div className="grid gap-0 lg:grid-cols-[1.2fr_1fr]">
          <div className="bg-surface2/40 p-4">
            <div className="overflow-hidden rounded-xl border border-border">
              <Media example={example} large />
            </div>
            {example.sourceAssetUrl && (
              <div className="mt-3">
                <p className="mb-1 text-xs text-muted">Input image (image-to-video)</p>
                <img src={example.sourceAssetUrl} alt="source" className="h-20 rounded-lg border border-border object-cover" />
              </div>
            )}
          </div>
          <ExampleDetailBody example={example} onRemix={onRemix} onClose={onClose} />
        </div>
      </div>
    </div>
  );
}

/** Shared detail body — used by the modal and the standalone example page. */
export function ExampleDetailBody({ example, onRemix, onClose }: { example: Example; onRemix?: (v: RemixValues) => void; onClose?: () => void }) {
  const [copied, setCopied] = useState(false);
  const copyPrompt = () => {
    navigator.clipboard?.writeText(example.prompt).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };
  const remix = () => {
    onRemix?.({ prompt: example.prompt, ...example.params });
    onClose?.();
  };

  return (
    <div className="space-y-4 p-5">
      <div className="flex flex-wrap gap-1.5">
        {example.techniques.map((t) => (
          <Badge key={t} tone="brand">
            {techniqueLabel(t)}
          </Badge>
        ))}
      </div>

      <div>
        <div className="mb-1 flex items-center justify-between">
          <span className="text-xs font-medium uppercase tracking-wide text-muted">Prompt</span>
          <button onClick={copyPrompt} className="inline-flex items-center gap-1 text-xs text-muted hover:text-fg">
            <Copy className="h-3.5 w-3.5" /> {copied ? "Copied" : "Copy"}
          </button>
        </div>
        <p className="rounded-lg bg-surface2/60 p-3 text-sm text-fg">{example.prompt}</p>
      </div>

      <div>
        <span className="text-xs font-medium uppercase tracking-wide text-muted">Parameters</span>
        <pre className="scrollbar-thin mt-1 overflow-x-auto rounded-lg bg-surface2/60 p-3 text-[13px] text-fg">
          <code>{JSON.stringify(example.params, null, 2)}</code>
        </pre>
      </div>

      {example.body && (
        <div>
          <span className="text-xs font-medium uppercase tracking-wide text-muted">How it's done</span>
          <p className="mt-1 text-sm leading-relaxed text-muted">{stripMd(example.body)}</p>
        </div>
      )}

      {onRemix && (
        <Button onClick={remix} className="w-full">
          <Wand2 className="h-4 w-4" /> Remix in playground
        </Button>
      )}
      <Link
        to={`/models/${example.model}/gallery/${example.id}`}
        className="inline-flex items-center gap-1 text-xs text-muted hover:text-fg"
      >
        Open example page <ExternalLink className="h-3.5 w-3.5" />
      </Link>
    </div>
  );
}

/* ── helpers ── */
function aspectToPadding(aspect: string): string {
  const [w, h] = aspect.split(":").map(Number);
  if (!w || !h) return "100%";
  return `${(h / w) * 100}%`;
}
/** Render the prose body as plain text (strip the light markdown we author). */
function stripMd(md: string): string {
  return md.replace(/\*\*/g, "").replace(/\*/g, "").replace(/`/g, "");
}
