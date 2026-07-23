/**
 * The generated-sample results for a model (covers, icons, examples from
 * `model_samples/`), as a clickable grid — every tile opens a lightbox showing
 * EVERYTHING that produced it: the output (animated webp loops for video
 * models), the conditioning/edit inputs with their own prompts, and the prompt
 * itself. Prev/next (buttons or ←/→) walks the whole set; ✕, backdrop, or Esc
 * closes. Data comes from the generated manifest (lib/model-samples.ts).
 */

import { useEffect, useRef, useState } from "react";
import { Check, ChevronLeft, ChevronRight, Copy, ExternalLink, X } from "lucide-react";
import { Badge, Button, Modal } from "@/components/ui";
import { sampleSetFor, type SampleKind } from "@/lib/model-samples";
import { cn } from "@/lib/cn";

type ShowcaseItem = {
  label: string;
  kind: SampleKind;
  prompt: string;
  output: string;
  inputs: string[];
  inputPrompts: string[];
};

const KIND_LABEL: Record<SampleKind, string> = {
  text_to_image: "Text to image",
  image_edit: "Image edit",
  text_to_video: "Text to video",
  image_to_video: "Image to video",
};

function itemsFor(slug: string): ShowcaseItem[] {
  const set = sampleSetFor(slug);
  if (!set) return [];
  const base: SampleKind = set.isVideo ? "text_to_video" : "text_to_image";
  const items: ShowcaseItem[] = [];
  if (set.cover && set.coverPrompt) {
    items.push({ label: "Cover art", kind: base, prompt: set.coverPrompt, output: set.cover, inputs: [], inputPrompts: [] });
  }
  if (set.icon && set.iconPrompt) {
    items.push({ label: "Icon", kind: base, prompt: set.iconPrompt, output: set.icon, inputs: [], inputPrompts: [] });
  }
  for (const [i, ex] of set.examples.entries()) {
    items.push({ label: `Example ${i + 1}`, kind: ex.kind, prompt: ex.prompt, output: ex.output, inputs: ex.inputs, inputPrompts: ex.inputPrompts });
  }
  return items;
}

/** True when a model has generated sample results to show. */
export function hasSampleShowcase(slug: string): boolean {
  return itemsFor(slug).length > 0;
}

export function SampleShowcase({ slug, modelName }: { slug: string; modelName: string }) {
  const items = itemsFor(slug);
  const [openIndex, setOpenIndex] = useState<number | null>(null);
  if (items.length === 0) return null;

  return (
    <div>
      <div className="mb-4 flex items-baseline justify-between">
        <h3 className="font-display text-xl font-semibold">Sample results</h3>
        <span className="text-xs text-muted">Generated with {modelName} through the live API — click any result for details</span>
      </div>
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4">
        {items.map((it, i) => (
          <button
            key={it.output}
            type="button"
            onClick={() => setOpenIndex(i)}
            title="View details"
            className="group relative block overflow-hidden rounded-xl border border-border bg-surface2/40 text-left"
          >
            <img src={it.output} alt={it.label} loading="lazy" className="aspect-square w-full object-cover transition duration-300 group-hover:scale-[1.03]" />
            <div className="pointer-events-none absolute inset-x-0 bottom-0 flex items-center justify-between gap-2 bg-gradient-to-t from-black/70 to-transparent px-3 pb-2 pt-8 opacity-0 transition group-hover:opacity-100">
              <span className="text-xs font-medium text-white">{it.label}</span>
              <span className="rounded bg-white/15 px-1.5 py-0.5 text-[10px] text-white">{KIND_LABEL[it.kind]}</span>
            </div>
          </button>
        ))}
      </div>
      {openIndex !== null && (
        <SampleLightbox items={items} modelName={modelName} initialIndex={openIndex} onClose={() => setOpenIndex(null)} />
      )}
    </div>
  );
}

function SampleLightbox({
  items,
  modelName,
  initialIndex,
  onClose,
}: {
  items: ShowcaseItem[];
  modelName: string;
  initialIndex: number;
  onClose: () => void;
}) {
  const [idx, setIdx] = useState(initialIndex);
  const [copied, setCopied] = useState(false);
  const flashTimer = useRef<number | null>(null);
  const item = items[Math.min(idx, items.length - 1)];
  const canPrev = idx > 0;
  const canNext = idx < items.length - 1;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowLeft") setIdx((i) => Math.max(i - 1, 0));
      if (e.key === "ArrowRight") setIdx((i) => Math.min(i + 1, items.length - 1));
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      if (flashTimer.current) window.clearTimeout(flashTimer.current);
    };
  }, [onClose, items.length]);

  const copyPrompt = async () => {
    try {
      await navigator.clipboard.writeText(item.prompt);
      setCopied(true);
      if (flashTimer.current) window.clearTimeout(flashTimer.current);
      flashTimer.current = window.setTimeout(() => setCopied(false), 1500);
    } catch {
      // clipboard blocked — the prompt is selectable text right below
    }
  };

  return (
    <Modal open onClose={onClose} className="max-w-4xl p-0">
      <div className="flex items-center justify-between border-b border-border px-5 py-3">
        <div className="flex items-center gap-2">
          <h3 className="font-display font-semibold">
            {item.label} <span className="text-muted">· {idx + 1} of {items.length}</span>
          </h3>
          <Badge tone="brand">{KIND_LABEL[item.kind]}</Badge>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={() => setIdx((i) => Math.max(i - 1, 0))}
            disabled={!canPrev}
            title="Previous (←)"
            className={cn("rounded-lg p-2 transition", canPrev ? "text-muted hover:bg-surface2 hover:text-fg" : "cursor-default text-muted/30")}
          >
            <ChevronLeft className="h-4 w-4" />
          </button>
          <button
            onClick={() => setIdx((i) => Math.min(i + 1, items.length - 1))}
            disabled={!canNext}
            title="Next (→)"
            className={cn("rounded-lg p-2 transition", canNext ? "text-muted hover:bg-surface2 hover:text-fg" : "cursor-default text-muted/30")}
          >
            <ChevronRight className="h-4 w-4" />
          </button>
          <button onClick={onClose} title="Close" className="rounded-lg p-2 text-muted transition hover:bg-surface2 hover:text-fg">
            <X className="h-4 w-4" />
          </button>
        </div>
      </div>

      <div className="grid gap-5 p-5 lg:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
        <div className="grid place-items-center rounded-xl bg-surface2/40 p-2">
          <img key={item.output} src={item.output} alt={item.label} className="max-h-[62vh] w-auto max-w-full rounded-lg object-contain" />
        </div>

        <div className="min-w-0 space-y-4">
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={() => void copyPrompt()}>
              {copied ? <Check className="h-3.5 w-3.5 text-emerald-500" /> : <Copy className="h-3.5 w-3.5" />}
              {copied ? "Copied" : "Copy prompt"}
            </Button>
            <Button variant="outline" size="sm" onClick={() => window.open(item.output, "_blank", "noopener")}>
              <ExternalLink className="h-3.5 w-3.5" />
              Open in new tab
            </Button>
          </div>

          <div>
            <div className="text-xs text-muted">Prompt</div>
            <p className="mt-1 max-h-36 overflow-y-auto whitespace-pre-wrap rounded-lg bg-surface2/60 px-3 py-2 text-sm text-fg">
              {item.prompt}
            </p>
          </div>

          {item.inputs.length > 0 && (
            <div>
              <div className="text-xs text-muted">
                {item.kind === "image_to_video" ? "Starting image" : "Edit inputs"} — generated first, then fed back in
              </div>
              <div className="mt-2 space-y-2">
                {item.inputs.map((u, i) => (
                  <div key={u} className="flex items-start gap-3 rounded-lg bg-surface2/60 p-2">
                    <img src={u} alt="" className="h-16 w-16 shrink-0 rounded-lg border border-border object-cover" />
                    <p className="min-w-0 flex-1 text-xs text-muted">{item.inputPrompts[i] || "Input image"}</p>
                  </div>
                ))}
              </div>
            </div>
          )}

          <div className="grid grid-cols-2 gap-2 text-sm">
            <Stat label="Model" value={modelName} />
            <Stat label="Task" value={KIND_LABEL[item.kind]} />
            <Stat label="Format" value={item.kind.endsWith("video") ? "animated webp · looping · muted" : "webp"} />
            <Stat label="Made via" value="live Foundation API" />
          </div>
        </div>
      </div>
    </Modal>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg bg-surface2/60 px-3 py-2">
      <div className="text-xs text-muted">{label}</div>
      <div className="truncate font-medium text-fg" title={value}>
        {value}
      </div>
    </div>
  );
}
