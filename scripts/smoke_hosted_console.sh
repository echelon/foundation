#!/usr/bin/env bash
set -euo pipefail

origin=${1:?Usage: bash scripts/smoke_hosted_console.sh CONSOLE_ORIGIN}
origin=${origin%/}
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

get_json() {
  curl -q --silent --show-error --fail --connect-timeout 5 --max-time 45 \
    --proto '=http,https' --header 'Accept: application/json' -- "$origin/$1"
}

check() {
  local route=$1 expected=$2
  shift 2
  if ! get_json "$route" | "$@" >/dev/null; then
    printf 'FAIL /%s must %s\n' "$route" "$expected" >&2
    return 1
  fi
  printf 'PASS /%s\n' "$route"
}

check ready 'report the gateway ready' jq -es 'length == 1 and .[0].ready == true'
check openapi.json 'match the release OpenAPI contract' \
  jq -es 'length == 2 and .[0] == .[1]' "$root/contracts/openapi.json" -
check catalog/artcraft-v1.json 'match the release ArtCraft catalog bytes' \
  cmp -s "$root/catalog/artcraft-v1.json" -
