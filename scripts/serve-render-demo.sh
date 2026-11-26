#!/usr/bin/env bash
set -euo pipefail
umask 077

if [[ -r /etc/secrets/tokencraft.env ]]; then
    set -a
    source /etc/secrets/tokencraft.env
    set +a
fi

: "${DATABASE_URL:?Set the Render Postgres external URL with sslmode=verify-full}"
: "${APP_BASE_URL:?Set the public HTTPS origin for the console}"
: "${SESSION_SECRET:?Set an independent 32-byte hex secret}"
: "${TOKENCRAFT_DEMO_PAYLOAD_KEY:?Set an independent 32-byte hex secret}"
: "${TOKENCRAFT_CONNECTOR_TOKEN_EXAMPLE:?Set an independent bearer token}"

export FOUNDATION_ENV=staging COOKIE_SECURE=true TOKENCRAFT_LISTEN_IP=0.0.0.0
export API_GATEWAY_PORT="${PORT:-10000}"
export TOKENCRAFT_CORE_CATALOG=/opt/tokencraft/catalog.json
export TOKENCRAFT_CONNECTORS=/opt/tokencraft/bindings.json

admin-cli migrate
state="$(mktemp -d)"
tokencraft example-connector "$state" &
connector=$!
api-gateway &
gateway=$!
trap 'kill "$gateway" "$connector" 2>/dev/null || true; wait || true' EXIT
trap 'exit 0' TERM INT
wait -n "$gateway" "$connector"
