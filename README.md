# TokenCraft

TokenCraft is a Rust gateway for routing AI requests across providers. It handles accounts, project keys, quotes, budgets, asynchronous requests, and artifact delivery. Providers connect through versioned HTTP bindings and operator-reviewed catalog entries.

The [ArtCraft reference catalog](catalog/artcraft-v1.json) describes supported provider plugins, model capabilities, and supplier list prices for each snapshot. A self-hosted gateway loads the operator's own catalog. Start with your own models or import selected ArtCraft presets with local bindings and rates. [Catalog setup](docs/provider-connectors.md#curate-your-own-catalog)

## Backend design

[Open the interactive design source](frontend/public/design/index.html). The console build serves it at `/design/`, with shareable links such as `/design/#overview/caller`. The diagrams identify observed code and proposed hosted architecture.

Follow the [API schema walkthrough](docs/api-schema.md) and use the console’s `/docs/api` for a searchable reference with one page per operation.

Build the browser console from `frontend/` with `npm ci` and `npm run build`. For Vercel, use `frontend` as the project root. The Vite build includes the design page and the application routes.

## Quickstart

Install Rust using the toolchain in [rust-toolchain.toml](rust-toolchain.toml) and the PostgreSQL 17 server tools. Put the server's bin directory or its `pg_config` on `PATH`.

```sh
cargo run --locked -p tokencraft -- dev
curl -fsS http://localhost:4000/ready
curl -fsS http://localhost:4000/v1/catalog/models
```

The launcher starts a private PostgreSQL cluster, applies migrations, starts the gateway, and runs the bundled synthetic connector. Commercial providers are installed as separate services and appear after their bindings and offers are added to the deployment catalog. The example generates synthetic PNGs without provider credentials, Python, or Node.js. Press Ctrl+C to stop the stack.

- [Backend setup and image walkthrough](docs/setup.md)
- [Start the demo with Docker Compose](docs/setup.md#start-with-docker-compose)
- [Start from a native package](docs/setup.md#start-from-a-native-package)
- [Start with your own catalog and provider bindings](docs/setup.md#start-with-a-reviewed-catalog)
- [Back up and restore dev state](docs/setup.md#back-up-and-restore-dev-state)
- [Run the fal, Kinovi, and OpenRouter plugins or configure your own compute](plugins/providers/README.md)
- [Configure a provider](docs/provider-connectors.md)
- [Prepare offers from the ArtCraft catalog](docs/provider-connectors.md#prepare-offers-from-the-artcraft-catalog)
- [OpenAPI and connector schema contracts](docs/api-contracts.md)

- [Draft and customize provider mappings](docs/provider-mapping.md)
- [Deployment and migration](docs/deployment.md)
- [Request diagnostics](docs/request-diagnostics.md)
- [Export project data to PostgreSQL](docs/customer-data.md)
- [Retain and restore project event history](docs/event-archives.md)
- [Retain and retrieve request content](docs/chat-history.md)
- [Rust SDK](sdk/rust/README.md) and [TypeScript SDK](sdk/typescript/README.md)
