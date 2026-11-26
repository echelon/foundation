import imageSuite from "../../content/benchmark-suites/image-1.json";
import { MODELS, MODEL_BY_SLUG, type Modality } from "./catalog";
import { cdnUrl } from "./gallery";

type ExampleModality = Exclude<Modality, "chat">;
type Settings = Record<string, string | number>;

export type SharedPrompt = { id: string; name: string; theme: string; prompt: string; criteria: string };
export type PromptSuite = {
  id: string;
  modality: ExampleModality;
  settings: Settings;
  summary: string;
  prompts: readonly SharedPrompt[];
};

const suites: Record<ExampleModality, PromptSuite> = {
  image: { ...imageSuite, modality: "image" },
  video: {
    id: "video-1", modality: "video", summary: "Text only · 6 seconds · 1280 × 720 · 1 clip",
    settings: { task: "text_to_video", reference_count: 0, output_count: 1, width: 1280, height: 720, duration_seconds: 6, tuning: "provider_default" },
    prompts: [
      { id: "tracking-shot", name: "Continuous tracking shot", theme: "Motion Consistency", criteria: "Motion and temporal consistency",
        prompt: "One continuous side-on shot. A red toy train with exactly two carriages moves from left to right along a tabletop track. The camera tracks alongside it at the same speed. Keep the locomotive and both carriages visible throughout. No cuts." },
      { id: "object-interaction", name: "Object interaction", theme: "Object Permanence", criteria: "Hand contact and object permanence",
        prompt: "A static close-up of a blue ceramic mug on a pale wooden desk. A single hand enters from the right, grasps the handle, lifts the mug briefly, and places it back in the same spot. Keep the desk and the mug unchanged between actions. No cuts." },
      { id: "orbit", name: "Camera orbit", theme: "Camera Geometry", criteria: "Parallax and consistent geometry",
        prompt: "A smooth camera move from front-left to front-right around a stationary copper desk lamp on a walnut plinth. A white wall and a vertical window frame sit behind the plinth. The lamp stays centered and keeps the same shape throughout. No cuts." },
      { id: "focus-pull", name: "Focus pull", theme: "Depth of Field", criteria: "Focus transition and exposure",
        prompt: "A locked camera frames a clear glass bottle in the foreground and a small yellow flower in the background. Start with the bottle sharp, then gradually pull focus until only the flower is sharp. Keep both objects and the window light steady. No cuts." },
    ],
  },
  "3d": {
    id: "mesh-1", modality: "3d", summary: "Text only · Textured triangle mesh · 1 GLB",
    settings: { task: "text_to_mesh", reference_count: 0, output_count: 1, format: "glb", topology: "triangle", material: "textured", tuning: "provider_default" },
    prompts: [
      { id: "stool", name: "Three-leg stool", theme: "Shape Accuracy", criteria: "Leg count, silhouette, and joins",
        prompt: "A single low red-oak stool with a round seat and exactly three tapered wooden legs. Show the grain on the seat and all three leg joins. Include the stool only, with no ground plane or extra objects." },
      { id: "lamp", name: "Thin lamp assembly", theme: "Thin Geometry", criteria: "Thin structures and separate parts",
        prompt: "A compact brass desk lamp with a circular base, two slender hinged arms, and a dark green metal shade. Make the shade visibly open at the bottom. Include the lamp only, with no ground plane or extra objects." },
      { id: "kettle", name: "Mixed-material kettle", theme: "Material Separation", criteria: "Material separation and surface detail",
        prompt: "A brushed stainless-steel kettle with a rounded body, a black ceramic lid knob, a curved black handle, and a short open spout. Keep each material distinct. Include the kettle only, with no ground plane or extra objects." },
      { id: "boot", name: "Textured hiking boot", theme: "Texture Detail", criteria: "Texture detail and asymmetric features",
        prompt: "A single left-foot hiking boot made of tan suede, with dark laces, six visible metal eyelets on each side, and a deep rubber tread. Include the boot only, with no ground plane or extra objects." },
    ],
  },
  world: {
    id: "world-1", modality: "world", summary: "Text only · Room scale · 1 Gaussian splat",
    settings: { task: "text_to_world", reference_count: 0, output_count: 1, format: "gaussian_splat", scale: "room", tuning: "provider_default" },
    prompts: [
      { id: "workshop", name: "Ceramics workshop", theme: "Room Layout", criteria: "Layout and viewpoint consistency",
        prompt: "A small, explorable ceramics workshop starting at the doorway. A wooden worktable stands in the center, shelving is against the left wall, and a wide north-facing window is on the far wall. Keep a clear path around all sides of the worktable." },
      { id: "reading-room", name: "Reading room", theme: "Depth and Occlusion", criteria: "Near-to-far depth and occlusion",
        prompt: "An explorable reading room viewed from its entrance, with a green armchair near the right wall, two tall bookcases along the left wall, and an archway to a small reading alcove directly ahead. Keep the floor clear between the entry and the alcove." },
      { id: "greenhouse", name: "Compact greenhouse", theme: "Interior Geometry", criteria: "Transparency and interior geometry",
        prompt: "A room-sized glass greenhouse viewed from the entrance. A narrow stone path runs down the center, long waist-high planters line both sides, and a closed glass door stands at the far end. Show a clear interior and realistic morning light." },
      { id: "studio", name: "Music recording studio", theme: "Material Continuity", criteria: "Material continuity and scene structure",
        prompt: "An explorable recording studio starting at the door, with a mixing desk in front, dark acoustic panels along both side walls, and a rectangular window into a small booth ahead. Place one microphone stand behind the window and keep the walking area clear." },
    ],
  },
};

const extensions: Record<ExampleModality, RegExp> = {
  image: /\.(png|jpe?g|webp)$/i,
  video: /\.(mp4|webm)$/i,
  "3d": /\.glb$/i,
  world: /\.(ply|spz)$/i,
};

export type SharedResult = {
  artifact: string;
  requestId: string;
  provider: string;
};
export type PublishedResults = ReadonlyMap<string, SharedResult>;

export function suiteForModel(modelSlug: string): PromptSuite | undefined {
  if (!Object.hasOwn(MODEL_BY_SLUG, modelSlug)) return undefined;
  const modality = MODEL_BY_SLUG[modelSlug].modality;
  return modality === "chat" ? undefined : suites[modality];
}

export function comparisonModels(suite: PromptSuite, current: string) {
  return MODELS.filter((model) => model.modality === suite.modality && model.slug !== current);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function matchesSettings(expected: Settings, actual: unknown): boolean {
  return isRecord(actual) && Object.keys(expected).length === Object.keys(actual).length
    && Object.entries(expected).every(([key, value]) => Object.hasOwn(actual, key) && actual[key] === value);
}

function identity(suiteId: string, promptId: string, model: string) {
  return `${suiteId}/${promptId}/${model}`;
}

function collectResults(entries: unknown[]) {
  const results = new Map<string, SharedResult>();
  const duplicates = new Set<string>();

  for (const entry of entries) {
    if (!isRecord(entry) || typeof entry.model !== "string" || typeof entry.request_id !== "string"
      || !/^[a-z0-9_-]{1,120}$/i.test(entry.request_id) || typeof entry.provider !== "string"
      || !entry.provider.trim() || typeof entry.artifact !== "string") continue;
    const suite = suiteForModel(entry.model);
    const scenario = suite?.prompts.find(({ id }) => id === entry.scenario);
    if (!suite || !scenario || entry.suite !== suite.id || entry.prompt !== scenario.prompt
      || !matchesSettings(suite.settings, entry.effective_settings)) continue;

    const key = identity(suite.id, scenario.id, entry.model);
    if (!entry.artifact.startsWith(`benchmarks/${key}/`) || !extensions[suite.modality].test(entry.artifact)
      || !entry.artifact.split("/").every((part) => /^[a-z0-9_-][a-z0-9_.-]*$/i.test(part))) continue;

    if (results.has(key) || duplicates.has(key)) {
      results.delete(key);
      duplicates.add(key);
      continue;
    }
    results.set(key, { artifact: cdnUrl(entry.artifact), requestId: entry.request_id, provider: entry.provider.trim() });
  }
  return results;
}

const manifests = import.meta.glob<unknown>("../../content/benchmarks/*.json", { eager: true, import: "default" });
const results = collectResults(Object.values(manifests));

export async function loadPublishedResults(suite: PromptSuite, modelSlug: string, signal: AbortSignal): Promise<PublishedResults | undefined> {
  const response = await fetch(cdnUrl(`benchmarks/${suite.id}/${modelSlug}/results.json`), {
    signal,
    credentials: "omit",
    cache: "no-cache",
  });
  if (!response.ok || !response.headers.get("content-type")?.includes("application/json")) return undefined;
  const entries: unknown = await response.json();
  if (!Array.isArray(entries) || entries.length > 16) return undefined;
  return collectResults(entries.filter((entry) => isRecord(entry) && entry.model === modelSlug && entry.suite === suite.id));
}

export function resultForPrompt(suite: PromptSuite, prompt: SharedPrompt, modelSlug: string, published?: PublishedResults): SharedResult | undefined {
  return (published ?? results).get(identity(suite.id, prompt.id, modelSlug));
}
