# Foundation — Frontend (mock)

A **frontend-only** build of the Foundation generative-media + LLM API gateway.
The backend is entirely stubbed — no Rust, no services — so we can see the UX and
evaluate how the seams fit before wiring anything real. Everything under
`src/lib/` (catalog, pricing, requests, auth) is mock data.

## Stack

- **Vite 6** + **React 19** + **TypeScript** (strict)
- **Tailwind CSS 3** with a CSS-variable token system
- **react-router-dom 7**, **lucide-react** icons

## Run

```bash
npm install
npm run dev        # http://localhost:3000
npm run build      # tsc + vite build
npm run typecheck
```

## Structure

The single Vite app is organized to mirror the intended Nx split so nothing has
to be undone when the real apps/libs are broken out:

```
src/
  lib/            → future shared libs
    catalog.ts    catalog-as-code: models, modalities, pricing, playground controls
    mock-api.ts   mock auth, balance, API keys, request ledger, endpoint reference
    theme.tsx     per-surface light/dark ThemeProvider
    cn.ts         class merge helper
  components/     shared UI primitives (ui.tsx) + ModelCard
  surfaces/       → future apps, one folder per surface
    marketing/    landing, pricing, docs (public)
    models/       catalog, model page (Playground / API / Examples / Pricing / Changelog)
    rankings/     the Rankings product — overview, per-media pages, model detail
    auth/         login, signup, password reset
    dashboard/    overview, requests, keys, billing, settings (mock logged-in)
    admin/        placeholder for the internal console
```

## Prompt Gallery (curated per-model examples)

The model page's **Examples** tab is a **Prompt Gallery** — curated examples showing what a
model can do and *how each effect is achieved* (the `awesome-*-prompts` pattern, productized).

- **Content-as-code**: examples are markdown files in `content/gallery/{model}/*.md`
  (frontmatter — model, task, prompt, params, technique tags, CDN asset key, provenance
  `job_id` — plus a prose "how it's done" body). Loaded via `import.meta.glob` in
  `src/lib/gallery.ts`; in production this is the `gallery.json` compile step
  (`design/subsystems/prompt_gallery.md`).
- **Assets** are fake images (`ffmpeg`-generated abstract art) and short videos (mp4 + poster)
  under `public/cdn/samples/...`, referenced by CDN object key exactly as the design specifies.
- **UI**: filterable masonry grid (by technique), hover-to-play video previews, a lightbox with
  the full prompt/params/prose, **"Remix in playground"** (loads the example's params into the
  playground form), and deep-linkable SEO example pages at `/models/:slug/gallery/:exampleId`.
- Prototyped for **Nano Banana 2** (8 image examples) and **Seedance 2.0** (6 video examples,
  incl. an image-to-video whose source is a Nano Banana still).

The playground also has a **request history** panel (like Fal): recent runs for the model with a
scope toggle — **My runs** vs **All account traffic** (everything that transited the account for
this model, including customers' end-users via `end_user`).

## Rankings (a headline feature)

`/rankings` is a full generative-media usage index — our answer to (and a superset of)
OpenRouter's LLM Rankings. Eight **verticals**: Image and Video first-class, then Language
(LLM), Speech, Transcription, Music, 3D, World. Data lives in `src/lib/rankings.ts`, shaped
like the design's rollup **cubes** (`design/core/model_rankings_and_reports.md` §5) so wiring
to `/v1/public/rankings*` later is a data-source swap, not a rebuild. Every number is derived
deterministically from a seed hash (no `Math.random`) so it renders identically on every reload.

OpenRouter-grade visuals:

- **Weekly stacked-bar chart** (top models over ~16 weeks, hoverable per-week breakdown) — the
  signature hero on the overview and every vertical page, with a media/metric switcher.
- **Week-over-week %** delta column on every leaderboard row, plus a 14-day sparkline.
- **Stacked-area** day-by-day share-over-time; **donut** market share by model and by lab.

Pages:

- `/rankings` — overview; stacked-bar hero, image & video leaderboards front and center,
  market-share donuts, movers of the week, vertical quick-nav.
- `/rankings/:media` — per-vertical page with a metric switch (e.g. video: total duration /
  seconds / requests) and sub-tabs **Leaderboard · By task · By category · By language\* ·
  Market share · Trends**. Image tasks: generate / edit / infill / style / upscale / background.
  Video categories: dialogue scene / action scene / b-roll / … (\*language tab for LLM / Speech /
  Transcription — programming languages like **Rust** + natural languages).
- `/rankings/models/:slug` — model detail: rank, share, WoW, trend, top tasks, top content
  categories, top languages, top apps, success rate, p50/p95 latency.

## Theming

Each **surface** owns its own day/night choice (persisted per surface in
`localStorage`). `ThemeProvider surface="…"` toggles a local `.dark` class, and
the token variables in `index.css` flip only within that wrapper. So marketing
can be light while the dashboard is dark, independently.

## Endpoints modeled (see `/docs` in-app and `mock-api.ts`)

- **Model-specific** — `POST /v1/models/{model}/generate[/{modality}]`, pins an exact model+version.
- **Modality** — `POST /v1/any/{image|video|3d|world}`, routes to the best (or named) model.
- **Cost twins** — every generate endpoint has a `…/cost` sibling that prices a request without running it.
- **Language** — OpenAI-compatible `POST /v1/chat/completions` (+ `/cost`).
- **Catalog / model pages** — public `GET /v1/models`, `/v1/models/{model}`, `/changelog`, `/examples`.

## Cost structure

Metering units: `per_image`, `per_video_second`, `per_mesh`, `per_splat`,
`per_million_tokens_in/out`. Requests draw down prepaid USD credits; tax is added
at top-up; invoices are numbered gaplessly. Details on the in-app `/docs` page.

## End-user attribution (flagged for the architecture docs)

Every request can carry an `end_user` identifier — the customer-of-our-customer.
The dashboard's **Requests** view has two scopes:

- **My requests** — what the logged-in user ran in the dashboard.
- **All account traffic** — *everything that transited the account*, including
  traffic your customers' end-users generated through your integration.

This is the piece the design docs still need to document; see the note at the top
of `src/lib/mock-api.ts`.
