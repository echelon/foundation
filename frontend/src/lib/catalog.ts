/**
 * Mock model catalog. Mirrors the design-doc "catalog as code" convention:
 * models, pricing and routing are hardcoded here (the future Rust catalog);
 * the DB only ever holds runtime state (jobs, balances, keys).
 *
 * Catalog slugs follow the `{provider}_{model}_{version?}` convention used in
 * the design docs, e.g. `google_nano_banana`, `bytedance_seedance_v2`.
 */

export type Modality = "image" | "video" | "3d" | "world" | "chat";

/** Pricing units the billing engine understands. */
export type PriceUnit =
  | "per_image"
  | "per_video_second"
  | "per_mesh"
  | "per_splat"
  | "per_million_tokens_in"
  | "per_million_tokens_out";

export type Price = { unit: PriceUnit; usd: number; note?: string };

/** A single playground control, rendered generically by the Playground surface. */
export type Control =
  | { key: string; label: string; type: "text"; placeholder?: string; default?: string; hint?: string }
  | { key: string; label: string; type: "textarea"; placeholder?: string; default?: string; hint?: string }
  | { key: string; label: string; type: "select"; options: { value: string; label: string }[]; default?: string; hint?: string }
  | { key: string; label: string; type: "slider"; min: number; max: number; step?: number; default: number; hint?: string }
  | { key: string; label: string; type: "toggle"; default?: boolean; hint?: string }
  | { key: string; label: string; type: "seed"; hint?: string };

export type ChangelogEntry = { version: string; date: string; note: string };

export type Model = {
  slug: string;
  name: string;
  provider: string;
  providerSlug: string;
  modality: Modality;
  tagline: string;
  description: string;
  tags: string[];
  released: string;
  status: "ga" | "preview" | "new" | "deprecated";
  pricing: Price[];
  controls: Control[];
  /** LLM-only: context window in tokens. */
  contextTokens?: number;
  changelog: ChangelogEntry[];
  /** Slugs of related models for the "Related" section. */
  related: string[];
};

export const MODALITY_LABEL: Record<Modality, string> = {
  image: "Image",
  video: "Video",
  "3d": "3D / Mesh",
  world: "3D World",
  chat: "Language",
};

/* ── Shared control presets ── */
const PROMPT: Control = { key: "prompt", label: "Prompt", type: "textarea", placeholder: "Describe what you want to generate…", hint: "required" };
const NEG_PROMPT: Control = { key: "negative_prompt", label: "Negative prompt", type: "text", placeholder: "What to avoid" };
const SEED: Control = { key: "seed", label: "Seed", type: "seed", hint: "blank = random" };
const ASPECT: Control = {
  key: "aspect_ratio",
  label: "Aspect ratio",
  type: "select",
  default: "1:1",
  options: [
    { value: "1:1", label: "1:1 · Square" },
    { value: "16:9", label: "16:9 · Landscape" },
    { value: "9:16", label: "9:16 · Portrait" },
    { value: "4:3", label: "4:3" },
    { value: "3:2", label: "3:2" },
  ],
};

/* ───────────────────────── Models ───────────────────────── */

export const MODELS: Model[] = [
  /* ── Google · Nano Banana image family ── */
  {
    slug: "google_nano_banana",
    name: "Nano Banana",
    provider: "Google",
    providerSlug: "google",
    modality: "image",
    tagline: "Fast, photoreal text-to-image with best-in-class prompt adherence.",
    description:
      "Nano Banana is Google's high-throughput image model tuned for photorealism and precise prompt following. Great default for product shots, concept art, and marketing creative.",
    tags: ["text-to-image", "image-to-image", "photoreal", "fast"],
    released: "2025-08-01",
    status: "ga",
    pricing: [{ unit: "per_image", usd: 0.02, note: "1024² · standard quality" }],
    controls: [PROMPT, NEG_PROMPT, ASPECT, { key: "num_images", label: "Images", type: "slider", min: 1, max: 4, default: 1 }, { key: "guidance", label: "Guidance", type: "slider", min: 1, max: 12, step: 0.5, default: 4, hint: "prompt strength" }, SEED],
    changelog: [{ version: "1.0", date: "2025-08-01", note: "Initial GA release." }],
    related: ["google_nano_banana_pro", "google_nano_banana_2", "openai_gpt_image_v1"],
  },
  {
    slug: "google_nano_banana_pro",
    name: "Nano Banana Pro",
    provider: "Google",
    providerSlug: "google",
    modality: "image",
    tagline: "Higher fidelity, better text rendering, and 2K output.",
    description:
      "Nano Banana Pro extends the base model with sharper detail, reliable in-image text, and native 2K generation. Best for hero images and typography-heavy creative.",
    tags: ["text-to-image", "typography", "2k", "high-fidelity"],
    released: "2025-11-10",
    status: "ga",
    pricing: [
      { unit: "per_image", usd: 0.05, note: "up to 2K · standard" },
      { unit: "per_image", usd: 0.09, note: "4K upscaled" },
    ],
    controls: [PROMPT, NEG_PROMPT, ASPECT, { key: "resolution", label: "Resolution", type: "select", default: "2k", options: [{ value: "1k", label: "1K" }, { value: "2k", label: "2K" }, { value: "4k", label: "4K (upscale)" }] }, { key: "num_images", label: "Images", type: "slider", min: 1, max: 4, default: 1 }, SEED],
    changelog: [
      { version: "1.0", date: "2025-11-10", note: "Pro tier launch with 2K + text rendering." },
    ],
    related: ["google_nano_banana", "google_nano_banana_2", "openai_gpt_image_v1_5"],
  },
  {
    slug: "google_nano_banana_2",
    name: "Nano Banana 2",
    provider: "Google",
    providerSlug: "google",
    modality: "image",
    tagline: "Next-gen architecture with stronger reasoning over complex scenes.",
    description:
      "Nano Banana 2 is a ground-up refresh with improved multi-subject composition, spatial reasoning, and reference-image conditioning. The new default for demanding scenes.",
    tags: ["text-to-image", "reference", "composition", "new"],
    released: "2026-06-20",
    status: "new",
    pricing: [{ unit: "per_image", usd: 0.04, note: "up to 2K · standard" }],
    controls: [PROMPT, NEG_PROMPT, ASPECT, { key: "reference_strength", label: "Reference strength", type: "slider", min: 0, max: 1, step: 0.05, default: 0.6, hint: "when a reference image is attached" }, { key: "num_images", label: "Images", type: "slider", min: 1, max: 4, default: 1 }, SEED],
    changelog: [{ version: "2.0", date: "2026-06-20", note: "New architecture, reference conditioning." }],
    related: ["google_nano_banana_pro", "openai_gpt_image_v2", "bytedance_seedance_v2"],
  },

  /* ── OpenAI · GPT Image family ── */
  {
    slug: "openai_gpt_image_v1",
    name: "GPT Image 1",
    provider: "OpenAI",
    providerSlug: "openai",
    modality: "image",
    tagline: "Instruction-following image generation and editing.",
    description:
      "GPT Image 1 excels at literal instruction following, inpainting, and edits driven by natural-language descriptions. Strong for iterative, conversational image workflows.",
    tags: ["text-to-image", "editing", "inpainting", "instruction"],
    released: "2025-04-23",
    status: "ga",
    pricing: [
      { unit: "per_image", usd: 0.04, note: "1024² · medium quality" },
      { unit: "per_image", usd: 0.08, note: "1024² · high quality" },
    ],
    controls: [PROMPT, { key: "quality", label: "Quality", type: "select", default: "medium", options: [{ value: "low", label: "Low" }, { value: "medium", label: "Medium" }, { value: "high", label: "High" }] }, ASPECT, { key: "num_images", label: "Images", type: "slider", min: 1, max: 4, default: 1 }, SEED],
    changelog: [{ version: "1.0", date: "2025-04-23", note: "GA release." }],
    related: ["openai_gpt_image_v1_5", "openai_gpt_image_v2", "google_nano_banana"],
  },
  {
    slug: "openai_gpt_image_v1_5",
    name: "GPT Image 1.5",
    provider: "OpenAI",
    providerSlug: "openai",
    modality: "image",
    tagline: "Faster edits, better hands and text, cheaper high-quality tier.",
    description:
      "An incremental upgrade to GPT Image 1 with improved anatomy, in-image text, and a cheaper high-quality tier. Drop-in compatible with v1 requests.",
    tags: ["text-to-image", "editing", "typography"],
    released: "2025-10-02",
    status: "ga",
    pricing: [
      { unit: "per_image", usd: 0.03, note: "1024² · medium quality" },
      { unit: "per_image", usd: 0.06, note: "1024² · high quality" },
    ],
    controls: [PROMPT, { key: "quality", label: "Quality", type: "select", default: "high", options: [{ value: "medium", label: "Medium" }, { value: "high", label: "High" }] }, ASPECT, { key: "num_images", label: "Images", type: "slider", min: 1, max: 4, default: 1 }, SEED],
    changelog: [{ version: "1.5", date: "2025-10-02", note: "Anatomy + text quality, cheaper high tier." }],
    related: ["openai_gpt_image_v1", "openai_gpt_image_v2", "google_nano_banana_pro"],
  },
  {
    slug: "openai_gpt_image_v2",
    name: "GPT Image 2",
    provider: "OpenAI",
    providerSlug: "openai",
    modality: "image",
    tagline: "Native multi-reference compositing and 4K output.",
    description:
      "GPT Image 2 introduces multi-reference compositing, consistent characters across generations, and native 4K. Built for brand and character-consistent pipelines.",
    tags: ["text-to-image", "characters", "4k", "reference", "new"],
    released: "2026-05-14",
    status: "new",
    pricing: [
      { unit: "per_image", usd: 0.06, note: "up to 2K" },
      { unit: "per_image", usd: 0.12, note: "4K" },
    ],
    controls: [PROMPT, { key: "resolution", label: "Resolution", type: "select", default: "2k", options: [{ value: "1k", label: "1K" }, { value: "2k", label: "2K" }, { value: "4k", label: "4K" }] }, ASPECT, { key: "num_images", label: "Images", type: "slider", min: 1, max: 4, default: 1 }, SEED],
    changelog: [{ version: "2.0", date: "2026-05-14", note: "Multi-reference compositing + 4K." }],
    related: ["openai_gpt_image_v1_5", "google_nano_banana_2", "bytedance_seedance_v2"],
  },

  /* ── ByteDance · Seedance video family ── */
  {
    slug: "bytedance_seedance_v2",
    name: "Seedance 2.0",
    provider: "ByteDance",
    providerSlug: "bytedance",
    modality: "video",
    tagline: "Cinematic text- and image-to-video with camera control.",
    description:
      "Seedance 2.0 generates smooth, cinematic clips from text or a starting image, with explicit camera-move control and strong temporal consistency.",
    tags: ["text-to-video", "image-to-video", "cinematic", "camera-control"],
    released: "2026-03-01",
    status: "ga",
    pricing: [{ unit: "per_video_second", usd: 0.12, note: "1080p · 24fps" }],
    controls: [
      PROMPT,
      { key: "duration", label: "Duration (s)", type: "slider", min: 2, max: 12, default: 5 },
      { key: "resolution", label: "Resolution", type: "select", default: "1080p", options: [{ value: "720p", label: "720p" }, { value: "1080p", label: "1080p" }] },
      { key: "camera", label: "Camera move", type: "select", default: "static", options: [{ value: "static", label: "Static" }, { value: "pan", label: "Pan" }, { value: "dolly", label: "Dolly" }, { value: "orbit", label: "Orbit" }, { value: "zoom", label: "Zoom" }] },
      ASPECT,
      SEED,
    ],
    changelog: [{ version: "2.0", date: "2026-03-01", note: "GA — camera control, 1080p." }],
    related: ["bytedance_seedance_v2_fast", "bytedance_seedance_v2_mini", "google_nano_banana_2"],
  },
  {
    slug: "bytedance_seedance_v2_fast",
    name: "Seedance 2.0 Fast",
    provider: "ByteDance",
    providerSlug: "bytedance",
    modality: "video",
    tagline: "Half the latency at a lower price, ideal for iteration.",
    description:
      "Seedance 2.0 Fast trades a little fidelity for roughly 2× faster generation and a lower per-second price. Perfect for rapid previews before a final render.",
    tags: ["text-to-video", "image-to-video", "fast", "preview"],
    released: "2026-03-01",
    status: "ga",
    pricing: [{ unit: "per_video_second", usd: 0.06, note: "1080p · 24fps" }],
    controls: [PROMPT, { key: "duration", label: "Duration (s)", type: "slider", min: 2, max: 10, default: 5 }, { key: "resolution", label: "Resolution", type: "select", default: "720p", options: [{ value: "720p", label: "720p" }, { value: "1080p", label: "1080p" }] }, ASPECT, SEED],
    changelog: [{ version: "2.0", date: "2026-03-01", note: "Fast variant launch." }],
    related: ["bytedance_seedance_v2", "bytedance_seedance_v2_mini"],
  },
  {
    slug: "bytedance_seedance_v2_mini",
    name: "Seedance 2.0 Mini",
    provider: "ByteDance",
    providerSlug: "bytedance",
    modality: "video",
    tagline: "Lightweight, low-cost clips for high-volume use.",
    description:
      "Seedance 2.0 Mini is the most economical tier — short, 720p clips at the lowest per-second price, tuned for high-volume automated pipelines.",
    tags: ["text-to-video", "economical", "high-volume"],
    released: "2026-04-10",
    status: "ga",
    pricing: [{ unit: "per_video_second", usd: 0.03, note: "720p · 24fps" }],
    controls: [PROMPT, { key: "duration", label: "Duration (s)", type: "slider", min: 2, max: 6, default: 4 }, ASPECT, SEED],
    changelog: [{ version: "2.0", date: "2026-04-10", note: "Mini variant launch." }],
    related: ["bytedance_seedance_v2", "bytedance_seedance_v2_fast"],
  },

  /* ── World Labs · Marble (3D world / gaussian splat) ── */
  {
    slug: "worldlabs_marble_v1",
    name: "Marble 1.0",
    provider: "World Labs",
    providerSlug: "worldlabs",
    modality: "world",
    tagline: "Generate explorable 3D worlds from a single image or prompt.",
    description:
      "Marble turns a prompt or image into a navigable 3D world exported as a Gaussian splat. Load it in any splat viewer or game engine for real-time exploration.",
    tags: ["image-to-3d", "gaussian-splat", "world", "scene"],
    released: "2025-12-05",
    status: "ga",
    pricing: [{ unit: "per_splat", usd: 0.9, note: "per generated world" }],
    controls: [PROMPT, { key: "scale", label: "World scale", type: "select", default: "room", options: [{ value: "object", label: "Object" }, { value: "room", label: "Room" }, { value: "landscape", label: "Landscape" }] }, SEED],
    changelog: [{ version: "1.0", date: "2025-12-05", note: "GA release." }],
    related: ["worldlabs_marble_v1_1", "tencent_hunyuan3d_v2_5"],
  },
  {
    slug: "worldlabs_marble_v1_1",
    name: "Marble 1.1",
    provider: "World Labs",
    providerSlug: "worldlabs",
    modality: "world",
    tagline: "Larger worlds, cleaner geometry, faster export.",
    description:
      "Marble 1.1 expands the maximum world size, reduces floaters in the splat, and cuts export time. Backwards-compatible with 1.0 requests.",
    tags: ["image-to-3d", "gaussian-splat", "world", "new"],
    released: "2026-06-01",
    status: "new",
    pricing: [{ unit: "per_splat", usd: 1.1, note: "per generated world" }],
    controls: [PROMPT, { key: "scale", label: "World scale", type: "select", default: "room", options: [{ value: "object", label: "Object" }, { value: "room", label: "Room" }, { value: "landscape", label: "Landscape" }, { value: "district", label: "District" }] }, { key: "quality", label: "Quality", type: "select", default: "balanced", options: [{ value: "fast", label: "Fast" }, { value: "balanced", label: "Balanced" }, { value: "high", label: "High" }] }, SEED],
    changelog: [{ version: "1.1", date: "2026-06-01", note: "Bigger worlds, cleaner geometry." }],
    related: ["worldlabs_marble_v1", "tencent_hunyuan3d_v2_5"],
  },

  /* ── Tencent · Hunyuan 3D (mesh) ── */
  {
    slug: "tencent_hunyuan3d_v2",
    name: "Hunyuan 3D 2.0",
    provider: "Tencent",
    providerSlug: "tencent",
    modality: "3d",
    tagline: "Image- and text-to-3D meshes with PBR textures.",
    description:
      "Hunyuan 3D 2.0 produces watertight meshes with PBR textures from a single image or prompt. Export GLB/OBJ ready for engines and DCC tools.",
    tags: ["image-to-3d", "text-to-3d", "mesh", "pbr"],
    released: "2025-09-18",
    status: "ga",
    pricing: [{ unit: "per_mesh", usd: 0.35, note: "textured mesh" }],
    controls: [PROMPT, { key: "topology", label: "Topology", type: "select", default: "triangle", options: [{ value: "triangle", label: "Triangle" }, { value: "quad", label: "Quad" }] }, { key: "texture", label: "Generate PBR texture", type: "toggle", default: true }, SEED],
    changelog: [{ version: "2.0", date: "2025-09-18", note: "GA — PBR textures." }],
    related: ["tencent_hunyuan3d_v2_5", "worldlabs_marble_v1_1"],
  },
  {
    slug: "tencent_hunyuan3d_v2_5",
    name: "Hunyuan 3D 2.5",
    provider: "Tencent",
    providerSlug: "tencent",
    modality: "3d",
    tagline: "Sharper geometry and higher-res textures.",
    description:
      "Hunyuan 3D 2.5 improves fine geometric detail and doubles texture resolution, with better handling of thin structures and multi-part objects.",
    tags: ["image-to-3d", "text-to-3d", "mesh", "high-res"],
    released: "2026-02-20",
    status: "ga",
    pricing: [{ unit: "per_mesh", usd: 0.5, note: "textured mesh · 2K texture" }],
    controls: [PROMPT, { key: "topology", label: "Topology", type: "select", default: "quad", options: [{ value: "triangle", label: "Triangle" }, { value: "quad", label: "Quad" }] }, { key: "texture_resolution", label: "Texture", type: "select", default: "2k", options: [{ value: "1k", label: "1K" }, { value: "2k", label: "2K" }] }, { key: "texture", label: "Generate PBR texture", type: "toggle", default: true }, SEED],
    changelog: [{ version: "2.5", date: "2026-02-20", note: "Sharper geometry, 2K textures." }],
    related: ["tencent_hunyuan3d_v2", "worldlabs_marble_v1_1"],
  },

  /* ── LLMs (advertised on OpenRouter) ── */
  {
    slug: "anthropic_claude_opus_4_8",
    name: "Claude Opus 4.8",
    provider: "Anthropic",
    providerSlug: "anthropic",
    modality: "chat",
    tagline: "Frontier reasoning, coding, and agentic workflows.",
    description:
      "Anthropic's most capable model for complex reasoning, long-horizon agentic tasks, and production coding. 1M-token context.",
    tags: ["reasoning", "coding", "agentic", "vision"],
    released: "2026-05-01",
    status: "ga",
    contextTokens: 1_000_000,
    pricing: [
      { unit: "per_million_tokens_in", usd: 5 },
      { unit: "per_million_tokens_out", usd: 25 },
    ],
    controls: [
      { key: "system", label: "System prompt", type: "textarea", placeholder: "You are a helpful assistant." },
      { key: "messages", label: "Message", type: "textarea", placeholder: "Ask anything…", hint: "required" },
      { key: "temperature", label: "Temperature", type: "slider", min: 0, max: 2, step: 0.1, default: 1 },
      { key: "max_tokens", label: "Max output tokens", type: "slider", min: 256, max: 16384, step: 256, default: 2048 },
    ],
    changelog: [{ version: "4.8", date: "2026-05-01", note: "Opus 4.8 release." }],
    related: ["anthropic_claude_haiku_4_5", "openai_gpt_5", "meta_llama_4_maverick"],
  },
  {
    slug: "anthropic_claude_haiku_4_5",
    name: "Claude Haiku 4.5",
    provider: "Anthropic",
    providerSlug: "anthropic",
    modality: "chat",
    tagline: "Fast, cheap, and surprisingly capable for its size.",
    description:
      "A low-latency, low-cost model for high-volume tasks — classification, extraction, routing, and lightweight chat — with strong quality per dollar.",
    tags: ["fast", "cheap", "high-volume"],
    released: "2025-10-01",
    status: "ga",
    contextTokens: 200_000,
    pricing: [
      { unit: "per_million_tokens_in", usd: 1 },
      { unit: "per_million_tokens_out", usd: 5 },
    ],
    controls: [
      { key: "system", label: "System prompt", type: "textarea", placeholder: "You are a helpful assistant." },
      { key: "messages", label: "Message", type: "textarea", placeholder: "Ask anything…", hint: "required" },
      { key: "temperature", label: "Temperature", type: "slider", min: 0, max: 2, step: 0.1, default: 1 },
      { key: "max_tokens", label: "Max output tokens", type: "slider", min: 256, max: 8192, step: 256, default: 1024 },
    ],
    changelog: [{ version: "4.5", date: "2025-10-01", note: "Haiku 4.5 release." }],
    related: ["anthropic_claude_opus_4_8", "openai_gpt_5_mini"],
  },
  {
    slug: "openai_gpt_5",
    name: "GPT-5",
    provider: "OpenAI",
    providerSlug: "openai",
    modality: "chat",
    tagline: "General-purpose frontier model with strong tool use.",
    description:
      "OpenAI's flagship general model, strong across reasoning, writing, and tool use, with a large context window and native structured outputs.",
    tags: ["reasoning", "tools", "vision"],
    released: "2025-08-07",
    status: "ga",
    contextTokens: 400_000,
    pricing: [
      { unit: "per_million_tokens_in", usd: 4 },
      { unit: "per_million_tokens_out", usd: 16 },
    ],
    controls: [
      { key: "system", label: "System prompt", type: "textarea", placeholder: "You are a helpful assistant." },
      { key: "messages", label: "Message", type: "textarea", placeholder: "Ask anything…", hint: "required" },
      { key: "temperature", label: "Temperature", type: "slider", min: 0, max: 2, step: 0.1, default: 1 },
      { key: "max_tokens", label: "Max output tokens", type: "slider", min: 256, max: 16384, step: 256, default: 2048 },
    ],
    changelog: [{ version: "5.0", date: "2025-08-07", note: "GPT-5 release." }],
    related: ["openai_gpt_5_mini", "anthropic_claude_opus_4_8", "google_gemini_3_pro"],
  },
  {
    slug: "openai_gpt_5_mini",
    name: "GPT-5 mini",
    provider: "OpenAI",
    providerSlug: "openai",
    modality: "chat",
    tagline: "Cost-efficient GPT-5 for everyday tasks.",
    description: "A smaller, cheaper GPT-5 variant tuned for latency-sensitive and high-volume workloads while keeping strong instruction following.",
    tags: ["fast", "cheap"],
    released: "2025-08-07",
    status: "ga",
    contextTokens: 400_000,
    pricing: [
      { unit: "per_million_tokens_in", usd: 0.6 },
      { unit: "per_million_tokens_out", usd: 2.4 },
    ],
    controls: [
      { key: "system", label: "System prompt", type: "textarea", placeholder: "You are a helpful assistant." },
      { key: "messages", label: "Message", type: "textarea", placeholder: "Ask anything…", hint: "required" },
      { key: "temperature", label: "Temperature", type: "slider", min: 0, max: 2, step: 0.1, default: 1 },
      { key: "max_tokens", label: "Max output tokens", type: "slider", min: 256, max: 8192, step: 256, default: 1024 },
    ],
    changelog: [{ version: "5.0", date: "2025-08-07", note: "GPT-5 mini release." }],
    related: ["openai_gpt_5", "anthropic_claude_haiku_4_5"],
  },
  {
    slug: "google_gemini_3_pro",
    name: "Gemini 3 Pro",
    provider: "Google",
    providerSlug: "google",
    modality: "chat",
    tagline: "Massive multimodal context with native video understanding.",
    description: "Gemini 3 Pro pairs a very large context window with strong multimodal understanding across text, images, and video.",
    tags: ["multimodal", "long-context", "vision"],
    released: "2026-01-15",
    status: "ga",
    contextTokens: 2_000_000,
    pricing: [
      { unit: "per_million_tokens_in", usd: 3.5 },
      { unit: "per_million_tokens_out", usd: 14 },
    ],
    controls: [
      { key: "system", label: "System prompt", type: "textarea", placeholder: "You are a helpful assistant." },
      { key: "messages", label: "Message", type: "textarea", placeholder: "Ask anything…", hint: "required" },
      { key: "temperature", label: "Temperature", type: "slider", min: 0, max: 2, step: 0.1, default: 1 },
      { key: "max_tokens", label: "Max output tokens", type: "slider", min: 256, max: 16384, step: 256, default: 2048 },
    ],
    changelog: [{ version: "3.0", date: "2026-01-15", note: "Gemini 3 Pro release." }],
    related: ["openai_gpt_5", "anthropic_claude_opus_4_8", "meta_llama_4_maverick"],
  },
  {
    slug: "meta_llama_4_maverick",
    name: "Llama 4 Maverick",
    provider: "Meta",
    providerSlug: "meta",
    modality: "chat",
    tagline: "Open-weight MoE model with excellent price-performance.",
    description: "Meta's open-weight mixture-of-experts model offering frontier-class quality at a fraction of closed-model prices. Great for self-serve scale.",
    tags: ["open-weight", "moe", "cheap"],
    released: "2025-04-05",
    status: "ga",
    contextTokens: 1_000_000,
    pricing: [
      { unit: "per_million_tokens_in", usd: 0.35 },
      { unit: "per_million_tokens_out", usd: 1.4 },
    ],
    controls: [
      { key: "system", label: "System prompt", type: "textarea", placeholder: "You are a helpful assistant." },
      { key: "messages", label: "Message", type: "textarea", placeholder: "Ask anything…", hint: "required" },
      { key: "temperature", label: "Temperature", type: "slider", min: 0, max: 2, step: 0.1, default: 1 },
      { key: "max_tokens", label: "Max output tokens", type: "slider", min: 256, max: 8192, step: 256, default: 1024 },
    ],
    changelog: [{ version: "4.0", date: "2025-04-05", note: "Llama 4 Maverick release." }],
    related: ["openai_gpt_5_mini", "anthropic_claude_haiku_4_5", "google_gemini_3_pro"],
  },
];

/* ── Lookups ── */
export const MODEL_BY_SLUG: Record<string, Model> = Object.fromEntries(MODELS.map((m) => [m.slug, m]));

export const PROVIDERS = Array.from(new Set(MODELS.map((m) => m.provider))).sort();

export function modelsByModality(modality: Modality) {
  return MODELS.filter((m) => m.modality === modality);
}

/** Human-friendly price line, e.g. "$0.02 / image" or "$5.00 / 1M input tokens". */
export function priceLabel(p: Price): string {
  const money = `$${p.usd.toFixed(p.usd < 0.1 ? 3 : 2)}`;
  switch (p.unit) {
    case "per_image":
      return `${money} / image`;
    case "per_video_second":
      return `${money} / video second`;
    case "per_mesh":
      return `${money} / mesh`;
    case "per_splat":
      return `${money} / world`;
    case "per_million_tokens_in":
      return `${money} / 1M input tokens`;
    case "per_million_tokens_out":
      return `${money} / 1M output tokens`;
  }
}
