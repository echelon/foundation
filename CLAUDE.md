# Foundation — repo-wide conventions

## Rust: NO telescopic positional args — use args structs

Functions must NOT take runs of positional same-typed parameters (especially `&str`/`Option<&str>`).
Call sites silently transpose them and the compiler can't help:

```rust
// FORBIDDEN — which one is the suffix? which is the prefix? nobody knows at the call site
pub fn object_key(folder: BucketFolder, hash: &str, prefix: Option<&str>,
                  suffix: Option<&str>, version: Option<u32>, extension: Option<&str>) -> String
```

Instead take a struct with named fields, so every call site names what it passes:

```rust
pub struct ObjectKeyParts<'a> {
  pub folder: BucketFolder,
  pub hash: &'a str,
  pub prefix: Option<&'a str>,
  pub suffix: Option<&'a str>,
  pub version: Option<u32>,
  pub extension: Option<&'a str>,
}
pub fn object_key(parts: ObjectKeyParts<'_>) -> String
```

Rules of thumb:
- Two adjacent params of the same type that could be swapped without a compile error → args struct
  (or newtypes). Passwords, emails, hashes, prefixes/suffixes are the classic offenders.
- 4+ params → args struct, period.
- The established pattern in this repo: `{Verb}{Noun}Args` / `New{Noun}` / `{Noun}Parts` structs —
  see `client` crates (`CreateTaskArgs`, `SendEmailArgs`, `UploadBytesArgs`), `db-queries`
  (`NewUser`, `NewSession`), `bucket_paths::ObjectKeyParts`.
- Typed ids (`ids` crate) already prevent id transposition — keep using them; never bare `String` ids.

## The api-gateway is NOT a CDN

Never serve asset bytes (images, media) from api-gateway handlers. APIs return **CDN URLs**
derived from stored path pieces (`bucket_paths`, files.md §2) — see the `MediaLinks`-style
response shape in `api-gateway/src/avatar_links.rs`: a `cdn_url` plus a `thumbnail_template`
with a `{WIDTH}` placeholder (Cloudflare Image Resizing `/cdn-cgi/image/...` in production;
in development there is no resizer, so the template is just the full-image URL). Bytes move
through R2/CDN only.

## Env & secrets

Services load convention-named env files at startup (see `crates/core/config`):
`crates/bin/{service}/config/{service}.{common,development,development-secrets,production}.env`
— `development-secrets` is gitignored. `cargo run -p {service}` must work with zero hand-supplied
env vars. Test credentials live in `external/credentials/*.txt` (gitignored).

## No `serde_json::Value` in request/response structs

Wire-facing structs (API request/response bodies, provider-client requests and responses)
must NEVER carry bare `serde_json::Value` / generic-JSON fields — a grab-bag field defeats
the type system exactly where drift hurts most. Type the payload:

- Closed unions get a real struct/enum (see `api-gateway`'s `AuditData` — every audit
  payload key is a typed optional field; adding a key means adding a field, never an
  ad-hoc `json!` at a write site — and `ParamAdjustmentBody`).
- Number-or-string provider quirks get an `#[serde(untagged)]` enum (see kinovi's
  `KinoviErrorCode`), never `Value`.
- The ONE sanctioned opaque boundary is a JSON **Schema** payload (arbitrary by nature):
  wrap it in `client_core::JsonSchemaDef`, which validates at construction.
- Database `jsonb` columns may decode to `Value` in `db-queries` row structs (that's the
  storage boundary) — but parse into typed shapes before anything wire-facing.

## Naming: version numbers in slugs are PLAIN — never "v"-prefixed

Model slugs (and any versioned identifier) write versions as bare numbers:
`google_nano_banana_2`, `openai_gpt_image_1_5` — **never** `google_nano_banana_v2`,
`openai_gpt_image_v1_5`. Do not use "v2"-style tokens anywhere. This applies to BOTH
catalogs (`crates/core/catalog` and `frontend/src/lib/catalog.ts`), the router, sample
directories under `model_samples/`, and docs.

## More conventions

- Database/query rules: `crates/db-queries/CLAUDE.md`
- Typed ids: `crates/core/ids/CLAUDE.md` and `crates/core/id-core/CLAUDE.md`
- Design docs (FOLLOW THEM): `external/claude_pairing/design/`
