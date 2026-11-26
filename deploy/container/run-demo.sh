#!/usr/bin/env bash
set -euo pipefail

children=()
stop() {
  if [[ ${children[0]+started} ]]; then
    kill -TERM "${children[@]}" 2>/dev/null || true
    wait "${children[@]}" 2>/dev/null || true
  fi
}
trap stop EXIT
trap 'exit 0' TERM INT

# Publish just the HTTP services. The launcher, PostgreSQL, and connector keep their loopback listeners.
socat TCP4-LISTEN:14000,reuseaddr,fork,max-children=64 TCP4:127.0.0.1:4000 &
children+=("$!")
if [[ -n "${TOKENCRAFT_DEMO_CONSOLE:-}" ]]; then
  socat TCP4-LISTEN:13000,reuseaddr,fork,max-children=64 TCP4:127.0.0.1:3000 &
  children+=("$!")
  tokencraft dev --state-dir /var/lib/postgresql/data/tokencraft --console-dir "$TOKENCRAFT_DEMO_CONSOLE" &
else
  tokencraft dev --state-dir /var/lib/postgresql/data/tokencraft &
fi
children+=("$!")

# A zero exit is also unexpected while the demo is serving.
wait -n "${children[@]}" || exit "$?"
exit 1
