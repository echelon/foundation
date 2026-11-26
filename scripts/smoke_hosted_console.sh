#!/usr/bin/env bash
set -euo pipefail

if (( $# != 2 )); then
  echo "Usage: bash scripts/smoke_hosted_console.sh CONSOLE_ORIGIN GATEWAY_ORIGIN" >&2
  exit 2
fi
console=${1%/}
gateway=${2%/}
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

get_json() {
  curl -q --silent --show-error --fail --connect-timeout 5 --max-time 45 \
    --proto '=http,https' --header 'Accept: application/json' -- "$1"
}

check() {
  local origin=$1 target=$2 route=$3 expected=$4
  shift 4
  if ! get_json "$origin/$route" | "$@" >/dev/null; then
    printf 'FAIL %s /%s must %s\n' "$target" "$route" "$expected" >&2
    return 1
  fi
  printf 'PASS %s /%s\n' "$target" "$route"
}

matches_contract() {
  jq -es 'length == 2 and .[0] == .[1]' "$root/contracts/openapi.json" -
}

result=0
check "$console" console ready 'report the gateway ready' jq -es 'length == 1 and .[0].ready == true' || result=1
check "$gateway" gateway ready 'report ready' jq -es 'length == 1 and .[0].ready == true' || result=1
check "$gateway" gateway openapi.json 'match the release OpenAPI contract' matches_contract || result=1
check "$console" console openapi.json 'serve the release OpenAPI contract' matches_contract || result=1
check "$console" console docs/openapi.json 'serve the release OpenAPI contract' matches_contract || result=1
check "$console" console catalog/artcraft-v1.json 'match the release ArtCraft catalog bytes' \
  cmp -s "$root/catalog/artcraft-v1.json" - || result=1
exit "$result"
