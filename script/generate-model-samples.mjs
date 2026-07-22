#!/usr/bin/env node
/**
 * Model-sample harness: generates each model's UI stand-in image set through the REAL
 * Foundation API (Bearer key, model-specific plane) and files it under
 *
 *   frontend/public/cdn/model_samples/image_models/{slug}/
 *     cover_image.webp          wide hero/background art
 *     icon_image.webp           square icon/emblem
 *     original_prompt.md        the cover + icon prompts & settings
 *     examples/
 *       example_1.output.webp   text-to-image showcase (+ .prompt.md)
 *       example_2.output.webp   text-to-image showcase (+ .prompt.md)
 *       example_3.input_*.webp  generated edit inputs
 *       example_3.output.webp   image-edit showcase (+ .prompt.md)
 *
 * Every image has a UNIQUE prompt; each model gets its own visual theme. Resumable:
 * anything already on disk is skipped, so re-running only fills gaps. Provider rate
 * limits (429 → failed job with provider_error) are retried with a cooldown.
 *
 * Usage:  node script/generate-model-samples.mjs [--only <slug>]
 *   FOUNDATION_API_BASE  (default http://localhost:4000)
 *   FOUNDATION_API_KEY   (default: external/credentials/foundation_api_key.txt)
 */

import { mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUT_ROOT = path.join(ROOT, "frontend/public/cdn/model_samples/image_models");
const API = process.env.FOUNDATION_API_BASE ?? "http://localhost:4000";

const KEY = (
  process.env.FOUNDATION_API_KEY ??
  (await readFile(path.join(ROOT, "external/credentials/foundation_api_key.txt"), "utf8"))
).trim();

/** Generation settings shared by every asset (webp, one image per request). */
const QUALITY = "medium";

/** One themed prompt set per model — no prompt is reused anywhere. */
const MODELS = [
  {
    slug: "openai_gpt_image_1",
    theme: "coastal watercolor",
    cover: {
      size: "1536x1024",
      prompt:
        "Sweeping watercolor panorama of a rugged coastline at golden hour, a white lighthouse on the far headland, translucent washes of teal and amber, soft paper texture, wide banner composition with calm negative space in the sky",
    },
    icon: {
      size: "1024x1024",
      prompt:
        "Minimal app icon of a lighthouse beam over stylized waves, flat geometric shapes, two-tone teal and warm gold on deep navy, centered, crisp edges, no text",
    },
    example_1: {
      size: "1024x1024",
      prompt:
        "A weathered rowboat resting on wet sand at low tide, loose watercolor style with granulating pigment, morning mist, gulls suggested with faint brushstrokes",
    },
    example_2: {
      size: "1024x1536",
      prompt:
        "Tall vertical watercolor of a tide pool seen from above, anemones and starfish in coral pink and sea green, ripples catching sunlight",
    },
    example_3: {
      input_1:
        "Simple watercolor study of an empty wooden pier stretching into a calm harbor, muted dawn palette",
      input_2:
        "Watercolor sketch of a small red sailboat with a single white sail, isolated on white paper",
      size: "1024x1024",
      prompt:
        "Combine these: moor the red sailboat from the second image at the end of the pier in the first image, matching the dawn light and adding soft reflections",
    },
  },
  {
    slug: "openai_gpt_image_1_mini",
    theme: "paper craft / origami",
    cover: {
      size: "1536x1024",
      prompt:
        "Layered paper-craft diorama of rolling hills with hot-air balloons and clouds cut from colored cardstock, long soft shadows, playful pastel palette, wide header composition",
    },
    icon: {
      size: "1024x1024",
      prompt:
        "App icon of an origami crane folded from coral-to-violet gradient paper, floating on a soft cream circle, subtle drop shadow, centered, no text",
    },
    example_1: {
      size: "1024x1024",
      prompt:
        "A paper-craft fox sitting under a cut-paper pine tree, visible layered cardboard edges, warm autumn colors, gentle studio lighting",
    },
    example_2: {
      size: "1024x1536",
      prompt:
        "Vertical paper-craft scene of a rocket launching through cotton-paper clouds with confetti stars, picture-book energy",
    },
    example_3: {
      input_1:
        "Paper-craft night scene: a small empty cardstock hill under a crescent moon cut from silver paper, deep blue layers",
      input_2: "Single origami rabbit folded from white paper with pink inner ears, on a plain background",
      size: "1024x1024",
      prompt:
        "Place the origami rabbit from the second image on top of the cardstock hill in the first image, gazing up at the silver moon, keeping the layered paper aesthetic",
    },
  },
  {
    slug: "openai_gpt_image_1_5",
    theme: "neon / synthwave",
    cover: {
      size: "1536x1024",
      prompt:
        "Ultra-wide synthwave cityscape at dusk, chrome towers with magenta and cyan neon reflected on wet streets, a low striped sun on the horizon, cinematic banner composition",
    },
    icon: {
      size: "1024x1024",
      prompt:
        "Glowing neon glyph of a thunderbolt inside a hexagon, hot pink and electric blue tubes on near-black, subtle bloom, app-icon composition, no text",
    },
    example_1: {
      size: "1024x1024",
      prompt:
        "A retro-futuristic arcade cabinet glowing in a dark room, CRT light spilling onto a checkered floor, dense neon signage reflections",
    },
    example_2: {
      size: "1024x1536",
      prompt:
        "Vertical shot of a neon-lit alley in the rain, holographic koi swimming through the air between fire escapes, cyberpunk mood",
    },
    example_3: {
      input_1: "Empty synthwave desert highway at night vanishing toward a low grid horizon, purple haze",
      input_2: "A single chrome retro-futuristic sports car with cyan underglow, studio shot on black",
      size: "1024x1024",
      prompt:
        "Put the chrome car from the second image speeding down the highway from the first image, with motion-blurred light trails, keeping the synthwave palette",
    },
  },
  {
    slug: "openai_gpt_image_2",
    theme: "cinematic architecture",
    cover: {
      size: "1536x1024",
      prompt:
        "Cinematic aerial photograph of a curved glass museum on a clifftop at blue hour, interior lights glowing, dramatic clouds, wide anamorphic banner framing, photorealistic",
    },
    icon: {
      size: "1024x1024",
      prompt:
        "Minimalist architectural emblem: a concrete arch casting a precise circular shadow, warm sunlight, photoreal macro, centered app-icon composition, no text",
    },
    example_1: {
      size: "1024x1024",
      prompt:
        "Photorealistic interior of a brutalist library atrium, shafts of dusty light across board-formed concrete, a single reader at a long oak table",
    },
    example_2: {
      size: "1024x1536",
      prompt:
        "Vertical photograph of a white marble spiral staircase shot straight up toward a glass oculus, vertigo perspective, soft daylight",
    },
    example_3: {
      input_1: "Photorealistic empty minimalist concrete plaza at golden hour, long shadows, mountains in haze",
      input_2: "Studio photograph of a bronze abstract sculpture of intertwined ribbons on a white background",
      size: "1024x1024",
      prompt:
        "Install the bronze sculpture from the second image in the center of the plaza from the first image, with golden-hour reflections and correct shadows, photorealistic",
    },
  },
];

/* ── API plumbing ── */

async function apiFetch(pathname, init = {}) {
  const res = await fetch(`${API}${pathname}`, {
    ...init,
    headers: { Authorization: `Bearer ${KEY}`, "Content-Type": "application/json", ...init.headers },
  });
  if (!res.ok) throw new Error(`${init.method ?? "GET"} ${pathname} → ${res.status}: ${await res.text()}`);
  return res.json();
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Enqueue one generation and wait for its output URL. Retries provider rate limits. */
async function generateImage({ model, prompt, size, imageInputs }) {
  const MAX_ATTEMPTS = 5;
  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
    const body = { prompt, n: 1, size, quality: QUALITY, output_format: "webp" };
    if (imageInputs?.length) body.image_inputs = imageInputs;
    const job = await apiFetch(`/v1/models/${model}/generate`, { method: "POST", body: JSON.stringify(body) });

    let j = job;
    while (j.status === "queued" || j.status === "running") {
      await sleep(3000);
      j = await apiFetch(`/v1/jobs/${job.id}`);
    }
    if (j.status === "succeeded" && j.outputs.length > 0) return { url: j.outputs[0].url, jobId: j.id };

    const msg = j.failure_message ?? j.status;
    if (attempt < MAX_ATTEMPTS && /rate limit/i.test(msg ?? "")) {
      console.log(`    rate limited — cooling down 70s (attempt ${attempt}/${MAX_ATTEMPTS})`);
      await sleep(70_000);
      continue;
    }
    throw new Error(`job ${j.id} ${j.status}: ${msg}`);
  }
}

async function download(url, dest) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`download ${url} → ${res.status}`);
  await writeFile(dest, Buffer.from(await res.arrayBuffer()));
}

/** Generate → download → return true; or skip when the file already exists. */
async function produce(dest, args, label) {
  if (existsSync(dest)) {
    console.log(`    ✓ ${label} (exists, skipped)`);
    return false;
  }
  console.log(`    … ${label}`);
  const { url, jobId } = await generateImage(args);
  await download(url, dest);
  console.log(`    ✓ ${label}  (${jobId})`);
  return true;
}

const promptMd = (title, prompt, extra = "") =>
  `# ${title}\n\n> ${prompt}\n\n- quality: ${QUALITY}\n- output_format: webp\n${extra}`;

/* ── Main ── */

const only = process.argv.includes("--only") ? process.argv[process.argv.indexOf("--only") + 1] : null;

for (const m of MODELS) {
  if (only && m.slug !== only) continue;
  const dir = path.join(OUT_ROOT, m.slug);
  const exDir = path.join(dir, "examples");
  await mkdir(exDir, { recursive: true });
  console.log(`\n■ ${m.slug} — ${m.theme}`);

  await produce(path.join(dir, "cover_image.webp"), { model: m.slug, ...m.cover }, "cover_image");
  await produce(path.join(dir, "icon_image.webp"), { model: m.slug, ...m.icon }, "icon_image");
  await writeFile(
    path.join(dir, "original_prompt.md"),
    `# ${m.slug} — sample-set prompts (${m.theme})\n\n## cover_image.webp (${m.cover.size})\n\n> ${m.cover.prompt}\n\n## icon_image.webp (${m.icon.size})\n\n> ${m.icon.prompt}\n\n- quality: ${QUALITY}\n- output_format: webp\n- generated via POST /v1/models/${m.slug}/generate\n`,
  );

  for (const n of [1, 2]) {
    const ex = m[`example_${n}`];
    await produce(
      path.join(exDir, `example_${n}.output.webp`),
      { model: m.slug, prompt: ex.prompt, size: ex.size },
      `example_${n}.output`,
    );
    await writeFile(
      path.join(exDir, `example_${n}.prompt.md`),
      promptMd(`${m.slug} — example ${n} (text-to-image, ${ex.size})`, ex.prompt),
    );
  }

  // example_3: generate the two edit inputs, then the edit itself using their CDN URLs.
  const ex3 = m.example_3;
  const in1 = path.join(exDir, "example_3.input_1.webp");
  const in2 = path.join(exDir, "example_3.input_2.webp");
  const inputUrls = [];
  for (const [dest, prompt, label] of [
    [in1, ex3.input_1, "example_3.input_1"],
    [in2, ex3.input_2, "example_3.input_2"],
  ]) {
    if (existsSync(dest) && existsSync(`${dest}.url`)) {
      inputUrls.push((await readFile(`${dest}.url`, "utf8")).trim());
      console.log(`    ✓ ${label} (exists, skipped)`);
    } else {
      console.log(`    … ${label}`);
      const { url, jobId } = await generateImage({ model: m.slug, prompt, size: "1024x1024" });
      await download(url, dest);
      await writeFile(`${dest}.url`, url); // keep the CDN ref so re-runs can redo the edit alone
      inputUrls.push(url);
      console.log(`    ✓ ${label}  (${jobId})`);
    }
  }
  await produce(
    path.join(exDir, "example_3.output.webp"),
    { model: m.slug, prompt: ex3.prompt, size: ex3.size, imageInputs: inputUrls },
    "example_3.output",
  );
  await writeFile(
    path.join(exDir, "example_3.prompt.md"),
    promptMd(
      `${m.slug} — example 3 (image edit, ${ex3.size})`,
      ex3.prompt,
      `- image_inputs: example_3.input_1.webp, example_3.input_2.webp\n\n## input_1 prompt\n\n> ${ex3.input_1}\n\n## input_2 prompt\n\n> ${ex3.input_2}\n`,
    ),
  );
}

console.log("\nDone.");
