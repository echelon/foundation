/**
 * Mock rollup data for the Rankings product. Shaped like the design's rollup
 * cubes (`model_rankings_and_reports.md` §5): the core cube (model × task ×
 * provider), the category cube (model × content_category), and derived views
 * (market share, movers, day-by-day trends).
 *
 * All numbers are DERIVED DETERMINISTICALLY from a string hash — no Math.random
 * — so the product renders identically on every reload and later swaps cleanly
 * to `/v1/public/rankings*` without reshaping the UI.
 */

/* ── Deterministic pseudo-random from a seed string (FNV-1a → [0,1)) ── */
function hashFloat(seed: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < seed.length; i++) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  // map to [0,1)
  return ((h >>> 0) % 100000) / 100000;
}

/** A stable value in [min,max] for (key). */
function pick(seed: string, min: number, max: number): number {
  return min + hashFloat(seed) * (max - min);
}

/* ── Media types (image & video first-class) ── */
export type MediaKey = "image" | "video" | "llm" | "audio" | "3d" | "world";

export type MediaConfig = {
  key: MediaKey;
  label: string;
  short: string;
  glyph: string;
  /** Metric label + short unit for output volume. */
  unit: string;
  unitShort: string;
  /** Rough total monthly output volume, for scaling model numbers. */
  monthlyVolume: number;
  tasks: { key: string; label: string }[];
  categories: { key: string; label: string }[];
  featured: boolean;
};

export type RankModel = {
  slug: string;
  name: string;
  owner: string;
  ownerSlug: string;
};

/* Curated, realistic model lists (names/owners from design catalog `models.md`). */
const IMAGE_MODELS: RankModel[] = [
  { slug: "google_nano_banana_pro", name: "Nano Banana Pro", owner: "Google", ownerSlug: "google" },
  { slug: "google_nano_banana_v2", name: "Nano Banana 2", owner: "Google", ownerSlug: "google" },
  { slug: "bytedance_seedream_v4_5", name: "Seedream 4.5", owner: "ByteDance", ownerSlug: "bytedance" },
  { slug: "bfl_flux_v2_pro", name: "FLUX.2 [pro]", owner: "Black Forest Labs", ownerSlug: "bfl" },
  { slug: "openai_gpt_image_v2", name: "GPT Image 2", owner: "OpenAI", ownerSlug: "openai" },
  { slug: "bytedance_seedream_v4", name: "Seedream 4.0", owner: "ByteDance", ownerSlug: "bytedance" },
  { slug: "openai_gpt_image_v1_5", name: "GPT Image 1.5", owner: "OpenAI", ownerSlug: "openai" },
  { slug: "ideogram_v4", name: "Ideogram 4.0", owner: "Ideogram", ownerSlug: "ideogram" },
  { slug: "bfl_flux_pro_v1_1_ultra", name: "FLUX 1.1 [pro] ultra", owner: "Black Forest Labs", ownerSlug: "bfl" },
  { slug: "recraft_v4_pro", name: "Recraft V4 Pro", owner: "Recraft", ownerSlug: "recraft" },
  { slug: "alibaba_qwen_image_v2_pro", name: "Qwen-Image 2 Pro", owner: "Alibaba", ownerSlug: "alibaba" },
  { slug: "midjourney_v8", name: "Midjourney V8", owner: "Midjourney", ownerSlug: "midjourney" },
  { slug: "luma_photon", name: "Luma Photon", owner: "Luma", ownerSlug: "luma" },
  { slug: "stability_sd_v3_5_large", name: "SD 3.5 Large", owner: "Stability", ownerSlug: "stability" },
];

const VIDEO_MODELS: RankModel[] = [
  { slug: "google_veo_v3_1", name: "Veo 3.1", owner: "Google", ownerSlug: "google" },
  { slug: "kling_video_v3_pro", name: "Kling 3.0 Pro", owner: "Kuaishou", ownerSlug: "kuaishou" },
  { slug: "bytedance_seedance_v2", name: "Seedance 2.0", owner: "ByteDance", ownerSlug: "bytedance" },
  { slug: "kling_video_v2_5_turbo_pro", name: "Kling 2.5 Turbo Pro", owner: "Kuaishou", ownerSlug: "kuaishou" },
  { slug: "openai_sora_v2_pro", name: "Sora 2 Pro", owner: "OpenAI", ownerSlug: "openai" },
  { slug: "bytedance_seedance_v1_5_pro", name: "Seedance 1.5 Pro", owner: "ByteDance", ownerSlug: "bytedance" },
  { slug: "alibaba_wan_v2_6", name: "Wan 2.6", owner: "Alibaba", ownerSlug: "alibaba" },
  { slug: "minimax_hailuo_v2_3", name: "Hailuo 2.3", owner: "MiniMax", ownerSlug: "minimax" },
  { slug: "runway_gen_v4_turbo", name: "Runway Gen-4 Turbo", owner: "Runway", ownerSlug: "runway" },
  { slug: "luma_ray_v3_2", name: "Luma Ray 3.2", owner: "Luma", ownerSlug: "luma" },
  { slug: "pixverse_v6", name: "PixVerse V6", owner: "PixVerse", ownerSlug: "pixverse" },
  { slug: "tencent_hunyuan_video_v1_5", name: "Hunyuan Video 1.5", owner: "Tencent", ownerSlug: "tencent" },
  { slug: "vidu_q3", name: "Vidu Q3", owner: "Vidu", ownerSlug: "vidu" },
];

const LLM_MODELS: RankModel[] = [
  { slug: "deepseek_v4_flash", name: "DeepSeek V4 Flash", owner: "DeepSeek", ownerSlug: "deepseek" },
  { slug: "xiaomi_mimo_v2_5", name: "MiMo-V2.5", owner: "Xiaomi", ownerSlug: "xiaomi" },
  { slug: "minimax_m3", name: "MiniMax M3", owner: "MiniMax", ownerSlug: "minimax" },
  { slug: "tencent_hy3", name: "Tencent Hy3", owner: "Tencent", ownerSlug: "tencent" },
  { slug: "zai_glm_v5_2", name: "GLM 5.2", owner: "Z.ai", ownerSlug: "zai" },
  { slug: "anthropic_claude_opus_v4_8", name: "Claude Opus 4.8", owner: "Anthropic", ownerSlug: "anthropic" },
  { slug: "anthropic_claude_sonnet_v5", name: "Claude Sonnet 5", owner: "Anthropic", ownerSlug: "anthropic" },
  { slug: "openai_gpt_v5_5", name: "GPT-5.5", owner: "OpenAI", ownerSlug: "openai" },
  { slug: "google_gemini_v3_1_pro", name: "Gemini 3.1 Pro", owner: "Google", ownerSlug: "google" },
  { slug: "meta_llama_v4_maverick", name: "Llama 4 Maverick", owner: "Meta", ownerSlug: "meta" },
  { slug: "moonshot_kimi_k2_6", name: "Kimi K2.6", owner: "Moonshot", ownerSlug: "moonshot" },
  { slug: "alibaba_qwen_v3_7_max", name: "Qwen3.7 Max", owner: "Alibaba", ownerSlug: "alibaba" },
  { slug: "xai_grok_v4_3", name: "Grok 4.3", owner: "xAI", ownerSlug: "xai" },
];

const AUDIO_MODELS: RankModel[] = [
  { slug: "elevenlabs_v3", name: "Eleven v3", owner: "ElevenLabs", ownerSlug: "elevenlabs" },
  { slug: "minimax_speech_v2_8_hd", name: "Speech 2.8 HD", owner: "MiniMax", ownerSlug: "minimax" },
  { slug: "cartesia_sonic_v3_5", name: "Sonic 3.5", owner: "Cartesia", ownerSlug: "cartesia" },
  { slug: "elevenlabs_music_v2", name: "Eleven Music v2", owner: "ElevenLabs", ownerSlug: "elevenlabs" },
  { slug: "openai_gpt_4o_mini_tts", name: "GPT-4o mini TTS", owner: "OpenAI", ownerSlug: "openai" },
  { slug: "google_lyria_v3", name: "Lyria 3", owner: "Google", ownerSlug: "google" },
  { slug: "cartesia_ink_v2", name: "Ink 2", owner: "Cartesia", ownerSlug: "cartesia" },
  { slug: "deepgram_nova_v3", name: "Nova-3", owner: "Deepgram", ownerSlug: "deepgram" },
];

const MESH_MODELS: RankModel[] = [
  { slug: "tencent_hunyuan3d_v3_1_pro", name: "Hunyuan 3D 3.1 Pro", owner: "Tencent", ownerSlug: "tencent" },
  { slug: "tripo_h3_1", name: "Tripo H3.1", owner: "Tripo", ownerSlug: "tripo" },
  { slug: "meshy_v6", name: "Meshy 6", owner: "Meshy", ownerSlug: "meshy" },
  { slug: "hyper3d_rodin_v2_5", name: "Rodin V2.5", owner: "Hyper3D", ownerSlug: "hyper3d" },
  { slug: "microsoft_trellis_v2", name: "TRELLIS.2", owner: "Microsoft", ownerSlug: "microsoft" },
  { slug: "tripo_v2_5", name: "Tripo v2.5", owner: "Tripo", ownerSlug: "tripo" },
  { slug: "meta_sam3d_objects", name: "SAM 3D Objects", owner: "Meta", ownerSlug: "meta" },
];

const WORLD_MODELS: RankModel[] = [
  { slug: "worldlabs_marble_v1_1", name: "Marble 1.1", owner: "World Labs", ownerSlug: "worldlabs" },
  { slug: "worldlabs_marble_v1_1_plus", name: "Marble 1.1 Plus", owner: "World Labs", ownerSlug: "worldlabs" },
  { slug: "tencent_hunyuan_world_v1", name: "HunyuanWorld 1.0", owner: "Tencent", ownerSlug: "tencent" },
  { slug: "spaitial_echo_v2", name: "SpAItial Echo-2", owner: "SpAItial", ownerSlug: "spaitial" },
  { slug: "worldlabs_marble_v1", name: "Marble 1.0", owner: "World Labs", ownerSlug: "worldlabs" },
];

export const MEDIA: Record<MediaKey, MediaConfig> = {
  image: {
    key: "image",
    label: "Image",
    short: "Image",
    glyph: "🖼",
    unit: "images generated",
    unitShort: "images",
    monthlyVolume: 2_140_000_000,
    featured: true,
    tasks: [
      { key: "text_to_image", label: "Generate" },
      { key: "image_edit", label: "Edit" },
      { key: "inpaint", label: "Infill" },
      { key: "image_to_image", label: "Style transfer" },
      { key: "upscale", label: "Upscale" },
      { key: "background_removal", label: "Background" },
    ],
    categories: [
      { key: "photorealistic", label: "Photorealistic" },
      { key: "portrait", label: "Portrait" },
      { key: "product", label: "Product" },
      { key: "character_art", label: "Character art" },
      { key: "anime_illustration", label: "Anime / illustration" },
      { key: "concept_art", label: "Concept art" },
      { key: "landscape", label: "Landscape" },
      { key: "logo_branding", label: "Logo / branding" },
      { key: "ui_mockup", label: "UI mockup" },
      { key: "infographic", label: "Infographic" },
    ],
  },
  video: {
    key: "video",
    label: "Video",
    short: "Video",
    glyph: "🎬",
    unit: "video-seconds generated",
    unitShort: "vid-sec",
    monthlyVolume: 61_800_000,
    featured: true,
    tasks: [
      { key: "text_to_video", label: "Text-to-video" },
      { key: "image_to_video", label: "Image-to-video" },
      { key: "reference_to_video", label: "Reference" },
      { key: "video_edit", label: "Video edit" },
      { key: "lipsync", label: "Lipsync" },
      { key: "avatar", label: "Avatar" },
    ],
    categories: [
      { key: "dialogue_scene", label: "Dialogue scene" },
      { key: "action_scene", label: "Action scene" },
      { key: "b_roll", label: "B-roll" },
      { key: "talking_head", label: "Talking head" },
      { key: "product_shot", label: "Product shot" },
      { key: "explainer_motion", label: "Explainer" },
      { key: "music_video", label: "Music video" },
      { key: "animation", label: "Animation" },
      { key: "vfx_shot", label: "VFX shot" },
      { key: "nature_landscape", label: "Nature / landscape" },
    ],
  },
  llm: {
    key: "llm",
    label: "Language",
    short: "LLM",
    glyph: "💬",
    unit: "tokens processed",
    unitShort: "tokens",
    monthlyVolume: 4_900_000_000_000,
    featured: false,
    tasks: [{ key: "chat", label: "Chat" }],
    categories: [
      { key: "coding", label: "Coding" },
      { key: "roleplay", label: "Roleplay" },
      { key: "extraction", label: "Extraction" },
      { key: "translation", label: "Translation" },
      { key: "summarization", label: "Summarization" },
      { key: "agentic", label: "Agentic" },
    ],
  },
  audio: {
    key: "audio",
    label: "Audio",
    short: "Audio",
    glyph: "🎧",
    unit: "audio-seconds generated",
    unitShort: "aud-sec",
    monthlyVolume: 38_400_000,
    featured: false,
    tasks: [
      { key: "text_to_speech", label: "Speech" },
      { key: "text_to_music", label: "Music" },
      { key: "speech_to_text", label: "Transcription" },
      { key: "text_to_sfx", label: "Sound FX" },
    ],
    categories: [
      { key: "narration", label: "Narration" },
      { key: "dialogue", label: "Dialogue" },
      { key: "music_track", label: "Music track" },
      { key: "sfx", label: "Sound FX" },
      { key: "podcast", label: "Podcast" },
    ],
  },
  "3d": {
    key: "3d",
    label: "3D / Mesh",
    short: "3D",
    glyph: "🧊",
    unit: "meshes generated",
    unitShort: "meshes",
    monthlyVolume: 4_200_000,
    featured: false,
    tasks: [
      { key: "image_to_mesh", label: "Image-to-3D" },
      { key: "text_to_mesh", label: "Text-to-3D" },
      { key: "retexture", label: "Retexture" },
      { key: "rigging", label: "Rigging" },
    ],
    categories: [
      { key: "character", label: "Character" },
      { key: "prop", label: "Prop" },
      { key: "environment", label: "Environment" },
      { key: "hard_surface", label: "Hard surface" },
      { key: "organic", label: "Organic" },
    ],
  },
  world: {
    key: "world",
    label: "3D World",
    short: "World",
    glyph: "🌐",
    unit: "worlds generated",
    unitShort: "worlds",
    monthlyVolume: 890_000,
    featured: false,
    tasks: [
      { key: "image_to_world", label: "Image-to-world" },
      { key: "text_to_world", label: "Text-to-world" },
      { key: "panorama_to_world", label: "Panorama" },
      { key: "world_edit", label: "World edit" },
    ],
    categories: [
      { key: "interior_scene", label: "Interior" },
      { key: "exterior_scene", label: "Exterior" },
      { key: "object", label: "Object" },
      { key: "landscape_world", label: "Landscape" },
    ],
  },
};

const MODELS_BY_MEDIA: Record<MediaKey, RankModel[]> = {
  image: IMAGE_MODELS,
  video: VIDEO_MODELS,
  llm: LLM_MODELS,
  audio: AUDIO_MODELS,
  "3d": MESH_MODELS,
  world: WORLD_MODELS,
};

export const MEDIA_ORDER: MediaKey[] = ["image", "video", "llm", "audio", "3d", "world"];

/* ── Period scaling ── */
export type Period = "day" | "week" | "month" | "all";
export const PERIODS: { key: Period; label: string }[] = [
  { key: "day", label: "Today" },
  { key: "week", label: "This week" },
  { key: "month", label: "This month" },
  { key: "all", label: "All time" },
];
const PERIOD_SCALE: Record<Period, number> = { day: 1 / 30, week: 7 / 30, month: 1, all: 11.4 };

export type Metric = "output" | "jobs" | "tokens";

/* ── Row computed for a leaderboard ── */
export type RankRow = {
  rank: number;
  model: RankModel;
  /** Base output-share of the media, 0..1. */
  share: number;
  /** Scaled to the current period/filter. */
  output: number;
  jobs: number;
  /** Week-over-week share change, percentage points (can be negative). */
  moverPct: number;
  successRate: number;
  p50: number;
  p95: number;
  sparkline: number[];
  firstSeen: string;
};

/** Intrinsic base weight for a model by list position (zipf-like decay). */
function baseWeight(media: MediaKey, idx: number): number {
  // top model heaviest; tuned so #1 ~ 18-24% and tail small
  return 1 / Math.pow(idx + 1.7, 1.12);
}

/** A per-(model,dimension) multiplier so different task/category boards reshuffle. */
function dimBoost(slug: string, dim: string): number {
  // 0.25..2.4 — some models dominate a task/category, others lag
  return 0.25 + hashFloat(`${slug}|${dim}`) * 2.15;
}

function sparkline(slug: string, media: MediaKey): number[] {
  const pts: number[] = [];
  let v = 40 + hashFloat(`${slug}|${media}|s0`) * 40;
  const drift = (hashFloat(`${slug}|${media}|drift`) - 0.45) * 6;
  for (let i = 0; i < 14; i++) {
    const noise = (hashFloat(`${slug}|${media}|d${i}`) - 0.5) * 14;
    v = Math.max(6, v + drift + noise);
    pts.push(Math.round(v));
  }
  return pts;
}

/**
 * Compute a leaderboard for a media type, optionally filtered to a task or a
 * content category, for a period, ranked by a metric. Returns rows sorted desc.
 */
export function leaderboard(
  media: MediaKey,
  opts: { task?: string; category?: string; period?: Period; metric?: Metric } = {},
): RankRow[] {
  const { task, category, period = "month" } = opts;
  const models = MODELS_BY_MEDIA[media];
  const dim = task ? `t:${task}` : category ? `c:${category}` : "all";

  // weighted values
  const weighted = models.map((m, idx) => {
    let w = baseWeight(media, idx);
    if (task) w *= dimBoost(m.slug, `t:${task}`);
    if (category) w *= dimBoost(m.slug, `c:${category}`);
    return { m, w };
  });
  const total = weighted.reduce((s, x) => s + x.w, 0);

  const scale = PERIOD_SCALE[period];
  const volume = MEDIA[media].monthlyVolume * scale * (task ? 0.4 : category ? 0.3 : 1);

  const rows: Omit<RankRow, "rank">[] = weighted.map(({ m, w }) => {
    const share = w / total;
    const output = Math.round(share * volume);
    const avgUnitsPerJob = media === "video" ? 6 : media === "llm" ? 2600 : media === "audio" ? 22 : 1;
    const jobs = Math.max(1, Math.round(output / avgUnitsPerJob));
    const moverPct = (hashFloat(`${m.slug}|${dim}|${period}|mv`) - 0.42) * 9;
    return {
      model: m,
      share,
      output,
      jobs,
      moverPct: Number(moverPct.toFixed(1)),
      successRate: Number(pick(`${m.slug}|sr`, 0.945, 0.998).toFixed(4)),
      p50: Math.round(pick(`${m.slug}|p50`, media === "video" ? 12000 : media === "llm" ? 600 : 1400, media === "video" ? 42000 : media === "llm" ? 5200 : 4200)),
      p95: Math.round(pick(`${m.slug}|p95`, media === "video" ? 48000 : media === "llm" ? 5200 : 4600, media === "video" ? 120000 : media === "llm" ? 18000 : 12000)),
      sparkline: sparkline(m.slug, media),
      firstSeen: firstSeenFor(m.slug),
    };
  });

  rows.sort((a, b) => b.share - a.share);
  return rows.map((r, i) => ({ ...r, rank: i + 1 }));
}

function firstSeenFor(slug: string): string {
  const months = ["2024-11", "2025-02", "2025-04", "2025-06", "2025-08", "2025-10", "2025-12", "2026-02", "2026-04", "2026-06"];
  return months[Math.floor(hashFloat(`${slug}|fs`) * months.length)] + "-01";
}

/* ── Market share by dimension ── */
export type ShareSlice = { key: string; label: string; ownerSlug?: string; share: number; output: number };

export function marketShare(media: MediaKey, by: "model" | "owner" | "provider", period: Period = "month"): ShareSlice[] {
  const rows = leaderboard(media, { period });
  if (by === "model") {
    return rows.map((r) => ({ key: r.model.slug, label: r.model.name, ownerSlug: r.model.ownerSlug, share: r.share, output: r.output }));
  }
  // aggregate by owner (owner==lab; provider approximated as owner's hosting for the mock)
  const agg = new Map<string, ShareSlice>();
  for (const r of rows) {
    const k = r.model.ownerSlug;
    const cur = agg.get(k) ?? { key: k, label: r.model.owner, ownerSlug: k, share: 0, output: 0 };
    cur.share += r.share;
    cur.output += r.output;
    agg.set(k, cur);
  }
  return Array.from(agg.values()).sort((a, b) => b.share - a.share);
}

/* ── Movers (biggest share change) ── */
export function movers(media: MediaKey, period: Period = "week"): { up: RankRow[]; down: RankRow[] } {
  const rows = leaderboard(media, { period });
  const sorted = [...rows].sort((a, b) => b.moverPct - a.moverPct);
  return { up: sorted.slice(0, 4), down: sorted.slice(-4).reverse() };
}

/* ── Day-by-day time series for a set of models (share over time) ── */
export type SeriesPoint = { day: string; [slug: string]: number | string };

const DAY_LABELS = ["Jul 06", "Jul 07", "Jul 08", "Jul 09", "Jul 10", "Jul 11", "Jul 12", "Jul 13", "Jul 14", "Jul 15", "Jul 16", "Jul 17", "Jul 18", "Jul 19"];

/** Stacked share-over-time for the top N models of a media type. */
export function shareTimeseries(media: MediaKey, topN = 6): { days: string[]; models: RankModel[]; series: SeriesPoint[] } {
  const rows = leaderboard(media, { period: "month" }).slice(0, topN);
  const series: SeriesPoint[] = DAY_LABELS.map((day, di) => {
    const point: SeriesPoint = { day };
    let sum = 0;
    const raw: number[] = rows.map((r) => {
      const base = r.share * 100;
      const wobble = (hashFloat(`${r.model.slug}|ts${di}`) - 0.5) * base * 0.5;
      return Math.max(0.5, base + wobble);
    });
    sum = raw.reduce((s, v) => s + v, 0);
    rows.forEach((r, i) => {
      point[r.model.slug] = Number(((raw[i] / sum) * 100).toFixed(1));
    });
    return point;
  });
  return { days: DAY_LABELS, models: rows.map((r) => r.model), series };
}

/* ── Per-model detail across all its media/tasks/categories ── */
export function findModel(slug: string): { media: MediaKey; model: RankModel } | undefined {
  for (const media of MEDIA_ORDER) {
    const m = MODELS_BY_MEDIA[media].find((x) => x.slug === slug);
    if (m) return { media, model: m };
  }
  return undefined;
}

export type ModelDetail = {
  media: MediaKey;
  model: RankModel;
  row: RankRow;
  topTasks: { key: string; label: string; share: number }[];
  topCategories: { key: string; label: string; share: number }[];
  topApps: { app: string; share: number }[];
};

const MOCK_APPS = ["Storyboard Studio", "AdForge", "Pixel Pipeline", "SceneSmith", "Creatify", "RenderRabbit", "MotionMint", "Frame0"];

export function modelDetail(slug: string): ModelDetail | undefined {
  const found = findModel(slug);
  if (!found) return undefined;
  const { media, model } = found;
  const row = leaderboard(media, { period: "month" }).find((r) => r.model.slug === slug)!;

  const topTasks = MEDIA[media].tasks
    .map((t) => ({ ...t, share: dimBoost(slug, `t:${t.key}`) }))
    .sort((a, b) => b.share - a.share)
    .slice(0, 5);
  const tt = topTasks.reduce((s, x) => s + x.share, 0);

  const topCategories = MEDIA[media].categories
    .map((c) => ({ ...c, share: dimBoost(slug, `c:${c.key}`) }))
    .sort((a, b) => b.share - a.share)
    .slice(0, 5);
  const tc = topCategories.reduce((s, x) => s + x.share, 0);

  const apps = MOCK_APPS.map((app) => ({ app, w: hashFloat(`${slug}|${app}`) }))
    .sort((a, b) => b.w - a.w)
    .slice(0, 5);
  const ta = apps.reduce((s, x) => s + x.w, 0);

  return {
    media,
    model,
    row,
    topTasks: topTasks.map((t) => ({ key: t.key, label: t.label, share: t.share / tt })),
    topCategories: topCategories.map((c) => ({ key: c.key, label: c.label, share: c.share / tc })),
    topApps: apps.map((a) => ({ app: a.app, share: a.w / ta })),
  };
}

/* ── Formatting helpers ── */
export function compact(n: number): string {
  if (n >= 1e12) return `${(n / 1e12).toFixed(1)}T`;
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)}B`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)}K`;
  return String(Math.round(n));
}

export function pct(n: number, digits = 1): string {
  return `${(n * 100).toFixed(digits)}%`;
}

/** Stable brand-ish color per owner for charts. */
export function ownerColor(ownerSlug: string): string {
  const hues = [258, 291, 190, 330, 150, 25, 210, 45, 0, 100, 270, 320];
  const h = Math.floor(hashFloat(ownerSlug) * hues.length);
  return `hsl(${hues[h]} 80% 60%)`;
}
