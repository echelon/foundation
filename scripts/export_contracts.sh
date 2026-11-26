#!/usr/bin/env bash
set -euo pipefail

out=${1:?Usage: bash scripts/export_contracts.sh OUTPUT_DIR [GATEWAY_URL]}
base=${2:-http://localhost:4000}
base=${base%/}
mkdir -p "$out"
work=$(mktemp -d "$out/.contracts.XXXXXXXX")
trap 'rm -rf "$work"' EXIT

fetch() {
  curl --silent --show-error --fail --connect-timeout 3 --max-time 15 \
    --proto '=http,https' --output "$work/$2" -- "$base/$1"
}

fetch openapi.json openapi.json
fetch v1/schemas/primitives connector-primitives.json
jq -e '(.openapi | startswith("3.1.")) and (.paths | length > 0)' "$work/openapi.json" >/dev/null
jq -e '(.["$id"] | startswith("urn:tokencraft:primitives:")) and (.["$defs"] | length > 0)' \
  "$work/connector-primitives.json" >/dev/null

# Match the newline added by the Rust export-openapi CLI.
printf '\n' >> "$work/openapi.json"
mv "$work/openapi.json" "$work/connector-primitives.json" "$out/"
