/**
 * Per-model request snippets for the model page's API section — cURL, JS fetch,
 * and Python requests. Mirrors the real endpoint shapes (`api_*` design docs):
 * chat models hit /v1/chat/completions, everything else the model-specific
 * /v1/models/{slug}/generate.
 */
import { type Model } from "./catalog";

const BASE = "https://api.foundation.ai";

/** Example body params per modality, kept short for readability. */
function bodyParams(model: Model): Record<string, unknown> {
  switch (model.modality) {
    case "video":
      return { prompt: "a serene mountain lake at dawn, slow drone push-in", duration: 5, resolution: "1080p" };
    case "3d":
      return { prompt: "a low-poly treasure chest, game asset" };
    case "world":
      return { prompt: "an explorable misty forest clearing" };
    case "image":
    default:
      return { prompt: "a serene mountain lake at dawn, cinematic", aspect_ratio: "16:9" };
  }
}

function isChat(model: Model) {
  return model.modality === "chat";
}

function endpoint(model: Model): string {
  return isChat(model) ? `${BASE}/v1/chat/completions` : `${BASE}/v1/models/${model.slug}/generate`;
}

function jsonBody(model: Model): Record<string, unknown> {
  if (isChat(model)) {
    return {
      model: model.slug,
      messages: [{ role: "user", content: "Explain quantum entanglement in one sentence." }],
      max_tokens: 512,
    };
  }
  return bodyParams(model);
}

/** result-access expression per language, tailored to the response shape. */
function resultJs(model: Model): string {
  return isChat(model) ? "data.choices[0].message.content" : "data.output[0].url";
}
function resultPy(model: Model): string {
  return isChat(model) ? 'data["choices"][0]["message"]["content"]' : 'data["output"][0]["url"]';
}

export function curlSnippet(model: Model): string {
  const body = JSON.stringify(jsonBody(model), null, 2)
    .split("\n")
    .map((l, i) => (i === 0 ? l : "  " + l))
    .join("\n");
  return `curl ${endpoint(model)} \\
  -H "Authorization: Bearer $FOUNDATION_KEY" \\
  -H "Content-Type: application/json" \\
  -d '${body}'`;
}

export function jsSnippet(model: Model): string {
  const body = JSON.stringify(jsonBody(model), null, 2)
    .split("\n")
    .map((l, i) => (i === 0 ? l : "    " + l))
    .join("\n");
  return `const res = await fetch("${endpoint(model)}", {
  method: "POST",
  headers: {
    Authorization: \`Bearer \${process.env.FOUNDATION_KEY}\`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify(${body}),
});

const data = await res.json();
console.log(${resultJs(model)});`;
}

export function pySnippet(model: Model): string {
  const body = JSON.stringify(jsonBody(model), null, 4)
    .split("\n")
    .map((l, i) => (i === 0 ? l : "    " + l))
    .join("\n")
    .replace(/: true/g, ": True")
    .replace(/: false/g, ": False");
  return `import os
import requests

res = requests.post(
    "${endpoint(model)}",
    headers={"Authorization": f"Bearer {os.environ['FOUNDATION_KEY']}"},
    json=${body},
)
data = res.json()
print(${resultPy(model)})`;
}
