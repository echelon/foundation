#!/usr/bin/env bash
set -euo pipefail
umask 077

api=${1:-http://localhost:4000}
worker=${2:-http://localhost:4001}
cookie=$(mktemp)
trap 'rm -f -- "$cookie"' EXIT

http() {
    curl --silent --show-error --noproxy '*' --connect-timeout 2 --max-time 8 "$@"
}

ready() {
    local origin="$1" attempt
    for ((attempt=0; attempt<60; attempt++)); do
        if http --fail "$origin/ready" 2>/dev/null | jq -e '.ready == true' >/dev/null; then
            return
        fi
        sleep 0.5
    done
    echo "Readiness did not pass on $origin" >&2
    return 1
}

ready "$api"
ready "$worker"
http --fail "$api/v1/catalog/models" |
    jq -e '[.models[].id] == ["runtime_image"]' >/dev/null
test "$(http --output /dev/null --write-out '%{http_code}' "$worker/v1/catalog/models")" = 404

signup=$(jq -nc --arg email "runtime-$(openssl rand -hex 8)@example.test" \
    --arg password "$(openssl rand -hex 24)" \
    '{email:$email,password:$password,display_name:"Gateway image smoke"}')
http --fail --cookie-jar "$cookie" -H 'Content-Type: application/json' \
    -d "$signup" "$api/v1/auth/signup" >/dev/null
account=$(http --fail --cookie "$cookie" "$api/v1/accounts" | jq -er '.accounts[0].account_id')
http --fail --cookie "$cookie" "$api/v1/setup/status?account_id=$account" |
    jq -e '.ready and .migrations == "ready" and .worker == "external"' >/dev/null

project=$(http --fail --cookie "$cookie" -H 'Content-Type: application/json' \
    -d '{"name":"Gateway image"}' "$api/v1/projects?account_id=$account" | jq -er '.id')
token=$(http --fail --cookie "$cookie" -H 'Content-Type: application/json' \
    -d '{"name":"Runtime probe","client_ref":"container_runtime","scopes":["requests:submit","requests:read","usage:read"]}' \
    "$api/v1/projects/$project/service-principals?account_id=$account" | jq -er '.token')

key=$(openssl rand -hex 16)
input='{"model":"runtime_image","workload":{"kind":"image","count":1,"aspect_ratio":"4:3"}}'
quote=$(http --fail -H "Authorization: Bearer $token" -H 'Content-Type: application/json' \
    -d "$input" "$api/v1/quotes/scoped")
payload=$(jq -ec 'select(.id != null and .input != null) | {quote_id:.id,input}' <<< "$quote")
submit() {
    http --fail -H "Authorization: Bearer $token" -H "Idempotency-Key: $key" \
        -H 'Content-Type: application/json' -d "$payload" "$api/v1/requests"
}

request_id=$(submit | jq -er '.id | select(test("^req_[A-Za-z0-9]+$"))')
test "$(submit | jq -er '.id')" = "$request_id"
state=
for ((attempt=0; attempt<60; attempt++)); do
    state=$(http --fail -H "Authorization: Bearer $token" "$api/v1/requests/$request_id" | jq -er '.status')
    case "$state" in
        succeeded) break ;;
        failed|canceled|awaiting_reconciliation) break ;;
    esac
    sleep 0.5
done
if [[ "$state" != succeeded ]]; then
    echo "Gateway image request ended as $state" >&2
    exit 1
fi
http --fail -H "Authorization: Bearer $token" "$api/v1/usage" |
    jq -e '.totals | .requests == 1 and .succeeded == 1 and .completed_outputs == 1
        and .estimated_usd_nanos == "1250000" and .mock_reported_usd_nanos == "1250000"' >/dev/null
echo "The API and separate worker completed one request and recorded the expected usage."
