import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui";
import { API_BASE_URL } from "@/lib/api";
import { withProject, type ProjectScope } from "@/lib/gateway";
import { comparisonApi, errorMessage, type ProviderAudio, type ProviderResult, type ProviderVideo } from "@/surfaces/comparison/api";

export function RequestResult({ scope, id, expiresAt, autoLoad = false }: {
  scope: ProjectScope; id: string; expiresAt: string; autoLoad?: boolean;
}) {
  const [data, setData] = useState<ProviderResult>();
  const [error, setError] = useState("");
  const [enabled, setEnabled] = useState(autoLoad);
  const [revision, setRevision] = useState(0);
  const [expired, setExpired] = useState(() => Date.parse(expiresAt) <= Date.now());
  const [copyNotice, setCopyNotice] = useState("");

  useEffect(() => {
    const remaining = Date.parse(expiresAt) - Date.now();
    if (remaining <= 0) {
      setExpired(true);
      return;
    }
    setExpired(false);
    const timer = window.setTimeout(() => { setData(undefined); setExpired(true); }, remaining);
    return () => window.clearTimeout(timer);
  }, [expiresAt]);

  useEffect(() => {
    if (!enabled || expired) return;
    const controller = new AbortController();
    setError("");
    comparisonApi.result(scope, id, controller.signal).then((value) => {
      if (!controller.signal.aborted) setData(value);
    }).catch((cause) => {
      if (!controller.signal.aborted) setError(errorMessage(cause));
    });
    return () => controller.abort();
  }, [scope, id, enabled, expired, revision]);

  if (expired) return <p className="text-sm text-muted">The stored result has expired.</p>;
  const result = enabled ? data?.result : undefined;
  const loading = enabled && !result && !error;

  async function copyText(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopyNotice("Copied.");
    } catch {
      setCopyNotice("Select the output text to copy it.");
    }
  }

  return (
    <section className="space-y-3" aria-label="Provider result">
      <div className="flex flex-wrap items-center gap-2">
        {!autoLoad && <Button size="sm" variant="outline" disabled={loading} onClick={() => {
          if (result) { setData(undefined); setEnabled(false); }
          else if (error) setRevision((value) => value + 1);
          else setEnabled(true);
        }}>{loading ? "Reading…" : result ? "Hide result" : "Read result"}</Button>}
        <p className="text-xs text-muted">Available until <time dateTime={expiresAt}>{new Date(expiresAt).toLocaleString()}</time></p>
      </div>
      {autoLoad && loading && <p role="status" className="text-sm text-muted">Loading output…</p>}
      {result && ("images" in result ? (
        <ul className={autoLoad ? "grid gap-3 sm:grid-cols-2" : "space-y-1 border border-border p-3 text-xs"}>
          {result.images.map((image, index) => {
            const url = image.artifact ? artifactDownloadUrl(id, image.artifact.slot, scope) : image.url;
            return <li key={index} className={autoLoad ? "min-w-0 overflow-hidden rounded-lg border border-border bg-surface2/50" : ""}>
              {autoLoad && <img className="max-h-80 w-full bg-surface2 object-contain" src={url} alt={`Generated image ${index + 1}`} loading="lazy" />}
              <div className={autoLoad ? "flex flex-wrap items-center justify-between gap-2 p-3 text-xs" : "flex flex-wrap gap-2"}>
                <a href={url} target="_blank" rel="noopener noreferrer" className="font-medium text-brand hover:underline">
                  {image.artifact ? "Download" : "Open"} image {index + 1}
                </a>
                <span className="text-muted">{image.width} × {image.height} · {image.content_type}</span>
              </div>
            </li>;
          })}
        </ul>
      ) : "video" in result ? (
        <TimedMediaResult media={result.video} scope={scope} id={id} />
      ) : "audio" in result ? (
        <TimedMediaResult media={result.audio} scope={scope} id={id} />
      ) : "mesh" in result ? (
        <div className="flex flex-wrap items-center gap-3 rounded-lg border border-border p-3 text-sm">
          <a href={artifactDownloadUrl(id, result.mesh.artifact.slot, scope)} className="font-medium text-brand hover:underline">Download GLB</a>
          <span className="text-muted">
            {result.mesh.mesh_count} {result.mesh.mesh_count === 1 ? "mesh" : "meshes"}
            {" · "}{result.mesh.primitive_count} {result.mesh.primitive_count === 1 ? "primitive" : "primitives"}
          </span>
        </div>
      ) : (
        <div className="space-y-2">
          <pre className="max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-lg border border-border bg-surface2/60 p-4 font-mono text-sm leading-6">{result.text}</pre>
          {autoLoad && <div className="flex items-center gap-2">
            <Button type="button" variant="outline" size="sm" onClick={() => void copyText(result.text)}>Copy text</Button>
            <p className="text-xs text-muted" role="status">{copyNotice}</p>
          </div>}
        </div>
      ))}
      {error && <div className="flex flex-wrap items-center gap-2">
        <p className="text-sm text-rose-600 dark:text-rose-300" role="alert">{error}</p>
        {autoLoad && <Button type="button" size="sm" variant="outline" onClick={() => setRevision((value) => value + 1)}>Retry output</Button>}
      </div>}
    </section>
  );
}

function TimedMediaResult({ media, scope, id }: { media: ProviderVideo | ProviderAudio; scope: ProjectScope; id: string }) {
  const url = artifactDownloadUrl(id, media.artifact.slot, scope);
  const kind = "width" in media ? "video" : "audio";
  const mime = kind === "video" ? "video/mp4" : "audio/wav";
  const details = "width" in media ? `${media.width} × ${media.height}` : `${media.sample_rate_hz / 1000} kHz · ${media.channels === 1 ? "Mono" : "Stereo"}`;
  const [source, setSource] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const controller = useRef<AbortController | null>(null);

  useEffect(() => () => controller.current?.abort(), []);
  useEffect(() => () => {
    if (source) URL.revokeObjectURL(source);
  }, [source]);

  async function preview() {
    const request = new AbortController();
    controller.current = request;
    setLoading(true);
    setError("");
    try {
      const response = await fetch(url, { credentials: "include", signal: request.signal });
      if (!response.ok || response.headers.get("Content-Type") !== mime) {
        throw new Error("Could not read the preview file.");
      }
      const body = await response.blob();
      if (body.size !== media.artifact.byte_length) throw new Error("Unexpected preview size.");
      if (!request.signal.aborted) setSource(URL.createObjectURL(body));
    } catch {
      if (!request.signal.aborted) setError(`The ${kind} preview could not be loaded.`);
    } finally {
      if (!request.signal.aborted) setLoading(false);
    }
  }

  return (
    <div className="space-y-2 border border-border p-3">
      <div className="flex flex-wrap items-center gap-3 text-xs">
        <a href={url} className="font-medium text-brand hover:underline">Download {kind}</a>
        <span className="text-muted">{details} · {(media.duration_ms / 1000).toFixed(2)}s</span>
        {!source && <Button type="button" size="sm" variant="outline" disabled={loading} onClick={() => void preview()}>
          {loading ? `Loading ${kind}…` : `Preview ${kind}`}
        </Button>}
      </div>
      {source && (kind === "video"
        ? <video aria-label="Generated video preview" className="w-full bg-black" src={source} controls playsInline />
        : <audio aria-label="Generated audio preview" className="w-full" src={source} controls />)}
      {error && <p className="text-xs text-rose-600 dark:text-rose-300" role="alert">{error}</p>}
    </div>
  );
}

function artifactDownloadUrl(id: string, slot: number, scope: ProjectScope) {
  return API_BASE_URL + withProject(`/v1/model-requests/${id}/artifacts/${slot}`, scope.project_id, scope.account_id);
}
