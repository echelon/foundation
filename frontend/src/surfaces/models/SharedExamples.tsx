import { useEffect, useRef, useState, type RefObject } from "react";
import { Link } from "react-router-dom";
import { ArrowUpRight, Check, Copy, Maximize2, Pause, Play, X } from "lucide-react";
import { Button, Select } from "@/components/ui";
import { ModelIcon } from "@/components/ModelArt";
import type { Model } from "@/lib/catalog";
import { cn } from "@/lib/cn";
import { defaultComparisonModels, modelComparisonUrl, suggestedExample } from "@/lib/model-comparison";
import { comparisonModels, loadPublishedResults, resultForPrompt, type PromptSuite, type PublishedResults, type SharedPrompt, type SharedResult } from "@/lib/model-examples";

const placeholderPath = "/benchmarks/placeholders";
const imageFrameClass = "h-[5.25rem] overflow-hidden rounded-md border border-border bg-surface2 sm:h-28 lg:h-[8rem]";
const iconButton = "grid h-7 w-7 shrink-0 place-items-center rounded-md px-0 text-muted transition hover:bg-surface2 hover:text-brand focus-visible:outline focus-visible:outline-2 focus-visible:outline-brand";

type Column = { model: Model; published?: PublishedResults };
type PreviewPlayback = "playing" | "paused" | "finished";

function usePublishedResults(suite: PromptSuite, slug: string | undefined) {
  const [loaded, setLoaded] = useState<{ suiteId: string; slug: string; results?: PublishedResults }>();
  useEffect(() => {
    if (!slug) return;
    const controller = new AbortController();
    loadPublishedResults(suite, slug, controller.signal).catch(() => undefined).then((results) => {
      if (!controller.signal.aborted) setLoaded({ suiteId: suite.id, slug, results });
    });
    return () => controller.abort();
  }, [suite, slug]);
  return loaded?.suiteId === suite.id && loaded.slug === slug ? loaded : undefined;
}

function usePreviewPlayback(gridRef: RefObject<HTMLOListElement | null>, enabled: boolean, setPlayback: (state: PreviewPlayback) => void) {
  const lastPlayed = useRef<HTMLVideoElement[]>([]);

  useEffect(() => {
    const grid = gridRef.current;
    if (!enabled || !grid) return;

    let videos: HTMLVideoElement[] = [];
    let phase: "cue" | "buffer" | "playing" = "cue";
    let active = true;

    const startWhenReady = () => {
      const next = Array.from(grid.querySelectorAll("video"));
      if (next.length !== videos.length || next.some((video, index) => video !== videos[index])) {
        // A comparison change or an unavailable result can replace part of the group.
        videos.forEach((video) => video.pause());
        videos = next;
        phase = "cue";
      }
      if (phase === "playing" || !videos.length || videos.some((video) => video.error && video.hasAttribute("data-matched"))) return;

      const playable = videos.filter((video) => !video.error);
      if (!playable.length) {
        setPlayback("paused");
        return;
      }
      if (phase === "cue") {
        const previous = lastPlayed.current;
        const restart = videos.length !== previous.length || videos.some((video, index) => video !== previous[index])
          || playable.every((video) => video.ended);
        if (restart) {
          for (const video of playable) {
            if (video.currentTime !== 0) video.currentTime = 0;
          }
        }
        phase = "buffer";
      }
      const starting = playable.filter((video) => !video.ended);
      if (playable.some((video) => video.seeking) || starting.some((video) => video.readyState < HTMLMediaElement.HAVE_FUTURE_DATA)) return;

      phase = "playing";
      const group = videos;
      lastPlayed.current = group;

      // Seeking can buffer again, so start only after both the rewound and new clips are ready.
      Promise.all(starting.map((video) => video.play())).catch(() => {
        if (!active || videos !== group) return;
        lastPlayed.current = [];
        setPlayback("paused");
      });
    };

    function finishWhenEnded() {
      if (videos.length && videos.every((video) => video.ended || video.error)) setPlayback("finished");
    }

    const changes = new MutationObserver(startWhenReady);
    changes.observe(grid, { childList: true, subtree: true });
    grid.addEventListener("canplay", startWhenReady, true);
    grid.addEventListener("seeked", startWhenReady, true);
    grid.addEventListener("error", startWhenReady, true);
    grid.addEventListener("ended", finishWhenEnded, true);
    startWhenReady();
    return () => {
      active = false;
      changes.disconnect();
      grid.removeEventListener("canplay", startWhenReady, true);
      grid.removeEventListener("seeked", startWhenReady, true);
      grid.removeEventListener("error", startWhenReady, true);
      grid.removeEventListener("ended", finishWhenEnded, true);
      videos.forEach((video) => video.pause());
    };
  }, [gridRef, enabled, setPlayback]);
}

export function SharedExamples({ model, suite }: { model: Model; suite: PromptSuite }) {
  const alternatives = comparisonModels(suite, model.slug);
  const [compare, setCompare] = useState("");
  const [selected, setSelected] = useState<SharedPrompt>();
  const [playback, setPlayback] = useState<PreviewPlayback>("playing");
  const gridRef = useRef<HTMLOListElement>(null);
  const other = alternatives.find((candidate) => candidate.slug === compare);
  const modelSet = other ? [model, other]
    : [model, ...defaultComparisonModels(model.modality).filter((item) => item.slug !== model.slug)].slice(0, 3);
  const primaryResults = usePublishedResults(suite, model.slug);
  const secondaryResults = usePublishedResults(suite, other?.slug);
  const columns: Column[] = [{ model, published: primaryResults?.results }];
  if (other) columns.push({ model: other, published: secondaryResults?.results });
  const videoSuite = suite.modality === "video";
  const resultsReady = Boolean(primaryResults && (!other || secondaryResults));
  usePreviewPlayback(gridRef, videoSuite && resultsReady && playback === "playing", setPlayback);
  const playbackLabel = playback === "playing" ? "Pause previews" : playback === "finished" ? "Replay previews" : "Play previews";

  return (
    <section aria-labelledby="shared-prompts-heading" className="space-y-3">
      <header className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-2 sm:gap-x-5">
        <div>
          <div className="flex items-center gap-1.5">
            <h2 id="shared-prompts-heading" className="font-display text-lg font-bold tracking-tight sm:text-xl">Shared prompts</h2>
            {videoSuite && (
              <button type="button" className={iconButton} aria-label={playbackLabel} title={playbackLabel}
                onClick={() => setPlayback(playback === "playing" ? "paused" : "playing")}>
                {playback === "playing" ? <Pause className="h-3.5 w-3.5" aria-hidden="true" /> : <Play className="h-3.5 w-3.5" aria-hidden="true" />}
              </button>
            )}
          </div>
          <p className="mt-0.5 font-mono text-[0.66rem] text-muted sm:text-xs">{suite.summary}</p>
        </div>
        {alternatives.length > 0 && (
          <label htmlFor="example-comparison-model" className="flex min-w-0 items-center gap-2 text-xs font-medium text-muted">
            <span id="example-comparison-label" className="sr-only shrink-0 sm:not-sr-only">Compare with</span>
            <Select id="example-comparison-model" aria-labelledby="example-comparison-label" value={compare} onChange={(event) => {
              setCompare(event.target.value);
              if (playback === "finished") setPlayback("playing");
            }} className="h-8 min-w-0 max-w-[8.25rem] text-xs sm:max-w-52">
              <option value="">Compare model</option>
              {alternatives.map(({ slug, name }) => <option key={slug} value={slug}>{name}</option>)}
            </Select>
          </label>
        )}
      </header>

      <Link to={modelComparisonUrl(modelSet, suggestedExample(modelSet))}
        className="inline-flex items-center gap-1.5 text-sm font-medium text-brand hover:underline">
        Compare model outputs and costs <ArrowUpRight className="h-3.5 w-3.5" aria-hidden />
      </Link>

      <ol ref={gridRef} className="grid grid-cols-2 gap-2.5 sm:gap-3" aria-label="Comparison cases">
        {suite.prompts.map((scenario) => (
          <li key={scenario.id} className="min-w-0 overflow-hidden rounded-xl border border-border bg-surface transition hover:border-brand/40">
            <div className="flex min-h-9 items-center justify-between gap-1.5 border-b border-border bg-surface2/25 pl-2 pr-1 sm:pl-3 sm:pr-2">
              <h3 className="min-w-0 py-1.5 font-display text-xs font-bold leading-tight sm:text-sm">{scenario.name}</h3>
              <div className="flex shrink-0 items-center">
                <CopyPrompt scenario={scenario} compact />
                <button type="button" className={iconButton} aria-label={`Open ${scenario.name} details`} title="Open full prompt and outputs" onClick={() => setSelected(scenario)}>
                  <Maximize2 className="h-3.5 w-3.5" aria-hidden="true" />
                </button>
              </div>
            </div>
            <div className="grid gap-2 p-2 sm:gap-2.5 sm:p-2.5 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.03fr)] lg:gap-3 lg:p-3">
              <div className="order-2 min-w-0 lg:order-1">
                <p className="line-clamp-3 text-[0.7rem] leading-[1.05rem] text-fg sm:line-clamp-4 sm:text-xs lg:line-clamp-none lg:text-[0.8rem] lg:leading-[1.12rem]">{scenario.prompt}</p>
                <p className="mt-1.5 hidden text-[0.67rem] leading-tight text-muted lg:block">{scenario.criteria}</p>
              </div>
              <div className={cn("order-1 grid min-w-0 gap-1.5 lg:order-2", other && "grid-cols-2")}>
                {columns.map(({ model: item, published }) => (
                  <Output key={item.slug} model={item} scenario={scenario} suite={suite} published={published} onOpen={() => setSelected(scenario)} showModel={Boolean(other)} />
                ))}
              </div>
            </div>
          </li>
        ))}
      </ol>
      {selected && <ScenarioDetail scenario={selected} suite={suite} columns={columns} onClose={() => setSelected(undefined)} />}
    </section>
  );
}

function ScenarioDetail({ scenario, suite, columns, onClose }: { scenario: SharedPrompt; suite: PromptSuite; columns: Column[]; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => element?.close();
  }, []);
  function dismiss() {
    dialog.current?.close();
    onClose();
  }

  return (
    <dialog ref={dialog} aria-labelledby="comparison-detail-title" onCancel={(event) => { event.preventDefault(); dismiss(); }}
      onClick={(event) => { if (event.currentTarget === event.target) dismiss(); }}
      className="m-auto max-h-[calc(100dvh-1.5rem)] w-[calc(100%-1.5rem)] max-w-5xl overflow-auto rounded-2xl border border-border bg-surface text-fg shadow-2xl backdrop:bg-black/65">
      <div className="space-y-4 p-4 sm:p-5">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 id="comparison-detail-title" className="font-display text-xl font-bold">{scenario.name}</h3>
            <p className="mt-0.5 text-xs text-muted">{scenario.criteria}</p>
          </div>
          <button type="button" aria-label="Close details" className={iconButton} onClick={dismiss}><X className="h-4 w-4" aria-hidden="true" /></button>
        </div>
        <div className="rounded-md border border-border bg-bg p-3">
          <p className="mb-2 text-sm leading-6">{scenario.prompt}</p>
          <CopyPrompt scenario={scenario} />
        </div>
        <div className={cn("grid gap-3", columns.length > 1 && "sm:grid-cols-2")}>
          {columns.map(({ model, published }) => <Output key={model.slug} model={model} scenario={scenario} suite={suite} published={published} showModel />)}
        </div>
      </div>
    </dialog>
  );
}

function CopyPrompt({ scenario, compact = false }: { scenario: SharedPrompt; compact?: boolean }) {
  const [feedback, setFeedback] = useState("");
  const copied = feedback === "Copied.";
  const Icon = copied ? Check : Copy;
  return (
    <div className={cn("items-center gap-2", compact ? "hidden sm:flex" : "flex flex-wrap")}>
      <Button type="button" variant={compact ? "ghost" : "outline"} size="sm"
        aria-label={compact ? `Copy ${scenario.name} prompt` : undefined}
        title={compact ? "Copy prompt" : undefined}
        className={compact ? iconButton : undefined} onClick={async () => {
        try {
          await navigator.clipboard.writeText(scenario.prompt);
          setFeedback("Copied.");
        } catch {
          setFeedback("Select the prompt to copy it.");
        }
      }}><Icon className="h-3.5 w-3.5" aria-hidden="true" />{!compact && "Copy prompt"}</Button>
      <span role="status" className={compact ? "sr-only" : "text-xs text-muted"}>{feedback}</span>
    </div>
  );
}

function Output({ model, scenario, suite, published, onOpen, showModel }: {
  model: Model;
  scenario: SharedPrompt;
  suite: PromptSuite;
  published?: PublishedResults;
  onOpen?: () => void;
  showModel: boolean;
}) {
  const result = resultForPrompt(suite, scenario, model.slug, published);
  const [failed, setFailed] = useState<string>();
  const matched = result && result.artifact !== failed ? result : undefined;
  const compact = Boolean(onOpen);
  const visual = suite.modality === "image" || suite.modality === "video";
  const label = `${scenario.name} from ${model.name}`;

  return (
    <section aria-label={`${model.name} output`} className={cn("min-w-0 space-y-1", !compact && "space-y-2 rounded-lg border border-border bg-surface2/25 p-3")}>
      {showModel && (
        <h4 title={model.name} className={cn("flex min-w-0 items-center gap-1.5 font-medium", compact ? "text-[0.65rem] sm:text-xs" : "text-sm")}>
          {!compact && <ModelIcon model={model} className="h-6 w-6" />}
          <span className="truncate">{model.name}</span>
        </h4>
      )}
      <div className={cn("relative min-w-0", suite.modality === "video" ? "aspect-video w-full overflow-hidden rounded-md bg-black" : compact && visual && imageFrameClass)}>
        {suite.modality === "video" ? (
          <video key={matched?.artifact ?? "placeholder"} src={matched?.artifact ?? `${placeholderPath}/flower.mp4`}
            poster={matched ? undefined : `${placeholderPath}/flower.jpg`} aria-label={matched ? label : "Stock placeholder clip"}
            data-matched={matched ? "" : undefined} controls={!compact} muted playsInline preload={compact ? "auto" : "none"}
            className="absolute inset-0 h-full w-full object-contain"
            onError={matched ? () => setFailed(matched.artifact) : undefined}
            onLoadedMetadata={matched ? ({ currentTarget }) => {
              const duration = suite.settings.duration_seconds;
              if (currentTarget.videoWidth !== suite.settings.width || currentTarget.videoHeight !== suite.settings.height
                || typeof duration !== "number" || !Number.isFinite(currentTarget.duration) || Math.abs(currentTarget.duration - duration) > 0.15) setFailed(matched.artifact);
            } : undefined} />
        ) : matched ? (
          <ResultMedia key={matched.artifact} label={label} result={matched} suite={suite} compact={compact} onUnavailable={() => setFailed(matched.artifact)} />
        ) : suite.modality === "image" ? (
          <img src={`${placeholderPath}/${scenario.id}.webp`} alt={`Illustrative preview for ${scenario.name}`} loading="lazy"
            className={compact ? "h-full w-full object-contain" : "max-h-[28rem] w-full rounded-md bg-surface2 object-contain"} />
        ) : (
          <p className="rounded-md border border-dashed border-border px-2 py-3 text-xs text-muted">Awaiting a matched run</p>
        )}
        {onOpen && visual && (
          <button type="button" onClick={onOpen} title="Open full prompt and outputs"
            aria-label={`Enlarge ${label}`} className="group absolute inset-0 flex items-end justify-end rounded-md p-1.5 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-brand">
            <Maximize2 className="h-5 w-5 rounded bg-black/60 p-1 text-white opacity-0 transition group-hover:opacity-100 group-focus-visible:opacity-100" aria-hidden="true" />
          </button>
        )}
      </div>
      {matched ? (
        <p className={cn("text-muted", compact ? "text-[0.65rem]" : "text-xs")}>Recorded output</p>
      ) : visual && (
        <div className="flex flex-wrap items-center gap-1.5 text-[0.65rem] text-muted sm:text-xs">
          <span className="rounded-sm bg-surface2 px-1.5 py-0.5">Placeholder</span>
          {!compact && <span>{result ? "Matched output unavailable" : suite.modality === "video" ? "Stock clip" : "Illustrative preview"}</span>}
          {compact && result && <span className="sr-only">Matched output unavailable</span>}
        </div>
      )}
    </section>
  );
}

function ResultMedia({ result, suite, label, compact, onUnavailable }: { result: SharedResult; suite: PromptSuite; label: string; compact: boolean; onUnavailable: () => void }) {
  if (suite.modality === "image") {
    return <img src={result.artifact} alt={label} loading="lazy" className={compact ? "h-full w-full object-contain" : "max-h-[28rem] w-full rounded-md bg-surface2 object-contain"}
      onError={onUnavailable} onLoad={({ currentTarget }) => {
        if (currentTarget.naturalWidth !== suite.settings.width || currentTarget.naturalHeight !== suite.settings.height) onUnavailable();
      }} />;
  }
  return (
    <a href={result.artifact} download className="inline-flex items-center gap-1.5 text-sm font-medium text-brand hover:underline">
      Download {suite.modality === "3d" ? "mesh" : "world"} <ArrowUpRight className="h-4 w-4" aria-hidden="true" />
    </a>
  );
}
