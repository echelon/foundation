/**
 * Prompt Gallery loader. Mirrors the design's content-as-code pipeline
 * (`design/subsystems/prompt_gallery.md`): curated examples are markdown files
 * in `content/gallery/{model}/*.md` (frontmatter + prose body), assets are CDN
 * object keys. Here we glob + parse them at build; in production this is the
 * `gallery.json` compile step that also powers `/v1/models/{model}/examples`.
 *
 * Frontmatter uses a small, fixed subset of YAML (scalars, one inline array,
 * one nested `params` block) so the demo needs no YAML dependency; the real
 * pipeline uses a proper parser.
 */

export type Example = {
  id: string;
  model: string;
  task: string;
  title: string;
  techniques: string[];
  aspect: string;
  prompt: string;
  params: Record<string, unknown>;
  /** CDN object key + resolved URL (design: key in content, host from config). */
  asset: string;
  assetUrl: string;
  poster?: string;
  posterUrl?: string;
  sourceAsset?: string;
  sourceAssetUrl?: string;
  isVideo: boolean;
  featured: boolean;
  weight: number;
  jobId?: string;
  body: string;
};

/** In the prototype the "CDN" is the public/ dir; production uses `https://{cdn_base}/{key}`. */
export function cdnUrl(key: string): string {
  return `/cdn/${key}`;
}

function coerce(v: string): unknown {
  const s = v.trim();
  if (s === "" || s === "null" || s === "~") return null;
  if (s === "true") return true;
  if (s === "false") return false;
  if (s.startsWith("[") && s.endsWith("]")) {
    return s
      .slice(1, -1)
      .split(",")
      .map((x) => x.trim())
      .filter(Boolean)
      .map((x) => stripQuotes(x));
  }
  if (/^-?\d+(\.\d+)?$/.test(s)) return Number(s);
  return stripQuotes(s);
}
function stripQuotes(s: string): string {
  if ((s.startsWith('"') && s.endsWith('"')) || (s.startsWith("'") && s.endsWith("'"))) return s.slice(1, -1);
  return s;
}

function parseExample(raw: string): Example | null {
  const lines = raw.replace(/\r\n/g, "\n").split("\n");
  if (lines[0].trim() !== "---") return null;
  let i = 1;
  const fm: Record<string, unknown> = {};
  const params: Record<string, unknown> = {};
  let inParams = false;
  for (; i < lines.length; i++) {
    const line = lines[i];
    if (line.trim() === "---") {
      i++;
      break;
    }
    if (/^\s+\S/.test(line)) {
      // indented → nested params entry
      const m = line.match(/^\s+([\w-]+):\s*(.*)$/);
      if (m && inParams) params[m[1]] = coerce(m[2]);
      continue;
    }
    const m = line.match(/^([\w-]+):\s*(.*)$/);
    if (!m) continue;
    const [, key, val] = m;
    if (key === "params" && val.trim() === "") {
      inParams = true;
      continue;
    }
    inParams = false;
    fm[key] = coerce(val);
  }
  const body = lines.slice(i).join("\n").trim();

  const asset = String(fm.asset ?? "");
  const poster = fm.poster ? String(fm.poster) : undefined;
  const sourceAsset = fm.source_asset ? String(fm.source_asset) : undefined;
  const isVideo = /\.(mp4|webm|mov)$/i.test(asset);

  return {
    id: String(fm.id),
    model: String(fm.model),
    task: String(fm.task),
    title: String(fm.title),
    techniques: (fm.techniques as string[]) ?? [],
    aspect: String(fm.aspect ?? "1:1"),
    prompt: String(fm.prompt ?? ""),
    params,
    asset,
    assetUrl: cdnUrl(asset),
    poster,
    posterUrl: poster ? cdnUrl(poster) : undefined,
    sourceAsset,
    sourceAssetUrl: sourceAsset ? cdnUrl(sourceAsset) : undefined,
    isVideo,
    featured: Boolean(fm.featured),
    weight: Number(fm.weight ?? 0),
    jobId: fm.job_id ? String(fm.job_id) : undefined,
    body,
  };
}

const RAW = import.meta.glob("../../content/gallery/**/*.md", { query: "?raw", import: "default", eager: true }) as Record<string, string>;

export const EXAMPLES: Example[] = Object.values(RAW)
  .map(parseExample)
  .filter((e): e is Example => e !== null);

export function examplesForModel(slug: string): Example[] {
  return EXAMPLES.filter((e) => e.model === slug).sort((a, b) => b.weight - a.weight);
}

export function exampleById(model: string, id: string): Example | undefined {
  return EXAMPLES.find((e) => e.model === model && e.id === id);
}

/** Distinct techniques used by a model's examples (for the filter chips). */
export function techniquesForModel(slug: string): string[] {
  const set = new Set<string>();
  for (const e of examplesForModel(slug)) e.techniques.forEach((t) => set.add(t));
  return Array.from(set).sort();
}

/** Pretty label for a technique slug ("cinematic-lighting" → "Cinematic lighting"). */
export function techniqueLabel(t: string): string {
  return t.replace(/-/g, " ").replace(/^\w/, (c) => c.toUpperCase());
}
