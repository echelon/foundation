# Request diagnostics

Use the durable ID returned by any request endpoint. A project key needs `requests:read` for the request's project and client.

| Endpoint | Evidence |
| --- | --- |
| `GET /v1/requests/{id}/trace` | Summary, recorded steps, logs, and the served provider and compute path |
| `GET /v1/requests/{id}/trace/baseline` | Median queue, elapsed, and publication times with the exact sample IDs |
| `GET /v1/requests/{id}/trace/otlp` | Redacted OpenTelemetry timing in OTLP/HTTP JSON |

The baseline uses earlier requests that succeeded before admission. It compares the same project, client, app, model, workload, API profile, and served route from the previous seven days. At most 50 matching requests contribute to the medians. Each timing reports the number with usable timestamps.

For example, use the original receipt ID with a project key.

```sh
curl -fsS -H "Authorization: Bearer $TOKENCRAFT_API_KEY" \
  "http://localhost:4000/v1/requests/$TOKENCRAFT_REQUEST_ID/trace"
```

The [Rust SDK](../sdk/rust/README.md#inspect-a-request) and [TypeScript SDK](../sdk/typescript/README.md) expose all three reads. Provider IDs come from the configured catalog, including custom connectors.
