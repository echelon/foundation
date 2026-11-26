#!/usr/bin/env bash
set -euo pipefail
umask 077

api=${1:-http://localhost:4000}
proof_dir=$(mktemp -d)
trap 'rm -f -- "$proof_dir/session.cookies" "$proof_dir/source-0.png" "$proof_dir/source-1.png" "$proof_dir/export.ndjson" "$proof_dir/restored.ndjson"; rmdir "$proof_dir"' EXIT

http() {
    curl --silent --show-error --noproxy '*' --connect-timeout 2 --max-time 15 "$@"
}

http --fail "$api/ready" | jq -e '.ready==true' >/dev/null
http --fail "$api/v1/schemas/primitives" \
  | jq -e '."$id" == "urn:tokencraft:primitives:1" and ."$defs".image_input.properties.count.maximum == 8' >/dev/null
http --fail "$api/v1/catalog/models" | jq -e '.models | map(.id) | index("example_image") != null' >/dev/null

cookie="$proof_dir/session.cookies"
email="dev-$(openssl rand -hex 8)@example.test"
password=$(openssl rand -hex 24)
signup=$(jq -nc --arg email "$email" --arg password "$password"     '{email:$email,password:$password,display_name:"Disposable TokenCraft smoke"}')
http --fail --cookie-jar "$cookie" -H 'Content-Type: application/json'     -d "$signup" "$api/v1/auth/signup" >/dev/null
account=$(http --fail --cookie "$cookie" "$api/v1/accounts" | jq -er '.accounts[0].account_id')
project=$(http --fail --cookie "$cookie" -H 'Content-Type: application/json'     -d '{"name":"Bundled connector verification"}' "$api/v1/projects?account_id=$account" | jq -er '.id')
provider_url="$api/v1/projects/$project/providers?account_id=$account"
inventory=$(http --fail --cookie "$cookie" "$provider_url")
jq -e '.offers | any(.id == "example_image" and .allowed_by_policy == true)' <<< "$inventory" >/dev/null
base_revision=$(jq -er '.base_revision' <<< "$inventory")
preview=$(jq -nc --arg revision "$base_revision" '{
  base_revision:$revision,
  change:{kind:"attach",name:"Mapping probe",provider_id:"mapping-probe",
    binding:{kind:"connector",name:"mapping_probe",origin:"https://connector.example.invalid/",
      revision:"v1",token_env:"TOKENCRAFT_CONNECTOR_TOKEN_MAPPING_PROBE"},
    models:[{id:"mapping_probe",model:"mapping_probe",upstream_model:"vendor/image",
      pricing:{kind:"image",usd_nanos_per_image:0,aspect_ratios:["1:1"]}}]}
}')
http --fail --cookie "$cookie" -H 'Content-Type: application/json' -d "$preview" "$provider_url" \
  | jq -e '.catalog.offers | any(.provider == "mapping-probe" and .upstream_model == "vendor/image")' >/dev/null
http --fail --cookie "$cookie" -X PUT -H 'Content-Type: application/json' \
  -d '{"expected_revision":0,"pause_all":false,"paused_providers":["example-custom"]}' \
  "$api/v1/operations/dispatch?account_id=$account" \
  | jq -e '.paused_providers == ["example-custom"]' >/dev/null
token=$(http --fail --cookie "$cookie" -H 'Content-Type: application/json'     -d '{"name":"Dev smoke","client_ref":"dev_smoke","scopes":["requests:submit","requests:read"]}'     "$api/v1/projects/$project/service-principals?account_id=$account" | jq -er '.token')

payload='{"model":"example_image","task":"text_to_image","prompt":"Two colored samples","count":2,"aspect_ratio":"4:3"}'
key="tokencraft-dev-$(openssl rand -hex 16)"
submit() {
    http --fail -H "Authorization: Bearer $token" -H 'Content-Type: application/json'         -H "Idempotency-Key: $key" -d "$payload" "$api/v1/image"
}
receipt=$(submit)
request_id=$(jq -er '.id' <<< "$receipt")
status_path=$(jq -er '.status_url' <<< "$receipt")
[[ "$status_path" =~ ^/v1/(model-)?requests/[A-Za-z0-9_-]+$ ]]
test "$(submit | jq -er '.id')" = "$request_id"
echo "Image request admitted and idempotent replay verified: $request_id"
for ((attempt=1; attempt<=150; attempt++)); do
    snapshot=$(http --fail -H "Authorization: Bearer $token" "$api$status_path")
    lifecycle=$(jq -er '.status' <<< "$snapshot")
    case "$lifecycle" in
        succeeded) break ;;
        failed|canceled|awaiting_reconciliation) echo "Request ended as $lifecycle" >&2; exit 1 ;;
    esac
    sleep 0.2
done
test "$lifecycle" = succeeded
echo "Gateway reported succeeded"
http --fail -H "Authorization: Bearer $token" "$api/v1/requests/$request_id/trace"   | jq -e '.evidence.summary.headline == "Request succeeded. Artifact publication completed." and (.evidence.routing.attempts | length == 1)' >/dev/null
peer_key="tokencraft-baseline-$(openssl rand -hex 16)"
peer=$(http --fail -H "Authorization: Bearer $token" -H 'Content-Type: application/json'     -H "Idempotency-Key: $peer_key" -d "$payload" "$api/v1/image" | jq -er '.id')
http --fail -H "Authorization: Bearer $token" "$api/v1/requests/$peer/trace/baseline"   | jq -e --arg sample "$request_id" '.request_ids == [$sample] and (.timings | any(.metric == "publication" and .samples == 1 and .median_ms >= 0))' >/dev/null
echo "Scoped request summary and artifact publication baseline verified"

verify_outputs() {
    local scope="$1" results artifact url output expected actual dimensions signature status
    results=$(http --fail -H "Authorization: Bearer $token" "$api$status_path/result")
    jq -e '.result.images | length == 2 and all(.width==64 and .height==48 and .content_type=="image/png")' <<< "$results" >/dev/null
    for slot in 0 1; do
        artifact=$(jq -e --argjson slot "$slot" '.result.images[$slot]' <<< "$results")
        url=$(jq -er '.url' <<< "$artifact")
        [[ "$url" =~ ^/v1/requests/[A-Za-z0-9_-]+/artifacts/[01]$ ]]
        expected=$(jq -er '.artifact.sha256' <<< "$artifact")
        output="$proof_dir/$scope-$slot.png"
        http --fail -H "Authorization: Bearer $token" "$api$url" -o "$output"
        if command -v shasum >/dev/null; then
            actual=$(shasum -a 256 "$output" | cut -d' ' -f1)
        else
            actual=$(sha256sum "$output" | cut -d' ' -f1)
        fi
        test "$expected" = "$actual"
        dimensions=$(od -v -An -j16 -N8 -tx1 "$output" | tr -d ' \n')
        signature=$(od -v -An -N8 -tx1 "$output" | tr -d ' \n')
        test "$signature" = 89504e470d0a1a0a
        test "$dimensions" = 0000004000000030
        status=$(http -o /dev/null -w '%{http_code}' "$api$url")
        test "$status" = 401
        echo "$scope output $slot: PNG 64x48, SHA-256 $actual, unauthenticated download HTTP $status"
    done
}
verify_outputs source

scope="account_id=$account&project_id=$project"
export_input=$(jq -nc --arg account "$account" --arg project "$project" \
    '{account_id:$account,project_id:$project,client_ref:"dev_smoke"}')
export_id=$(http --fail --cookie "$cookie" -H 'Content-Type: application/json' \
    -d "$export_input" "$api/v1/exports" | jq -er '.id | select(test("^exp_[A-Za-z0-9]+$"))')
http --fail --cookie "$cookie" "$api/v1/exports/$export_id/data?$scope" -o "$proof_dir/export.ndjson"
jq -se 'length > 0 and all(.client_ref == "dev_smoke")' "$proof_dir/export.ndjson" >/dev/null

archive_input=$(jq -nc --arg account "$account" --arg project "$project" --arg export "$export_id" \
    '{account_id:$account,project_id:$project,export_id:$export}')
archive_id=$(http --fail --cookie "$cookie" -H 'Content-Type: application/json' \
    -d "$archive_input" "$api/v1/archives" | jq -er '.id | select(test("^arc_[A-Za-z0-9]+$"))')

wait_for_archive() {
    local target="$1" state attempt
    for ((attempt=1; attempt<=100; attempt++)); do
        state=$(http --fail --cookie "$cookie" "$api/v1/archives/$archive_id?$scope" | jq -er '.state')
        case "$state" in
            "$target") return ;;
            *failed) echo "Archive ended as $state" >&2; return 1 ;;
        esac
        sleep 0.2
    done
    echo "Archive did not reach $target" >&2
    return 1
}

wait_for_archive stored
http --fail --cookie "$cookie" -X POST "$api/v1/archives/$archive_id/restore?$scope" >/dev/null
wait_for_archive available
http --fail --cookie "$cookie" "$api/v1/archives/$archive_id/data?$scope" -o "$proof_dir/restored.ndjson"
cmp "$proof_dir/export.ndjson" "$proof_dir/restored.ndjson"
echo "Client-scoped encrypted archive restored with identical export bytes"
