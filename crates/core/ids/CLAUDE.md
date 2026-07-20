# ids — the concrete strongly-typed id types

Every entity id in Foundation is a distinct type defined here (`UserId`, `AccountId`, `SessionId`,
`InvitationId`, `SsoConnectionId`, `JobId`, …). This crate is the one the rest of the workspace
depends on for ids. The *machinery* (minting, validation, the `define_id!` macro) lives in `id-core`.

## The rule: NEVER stringly-typed ids

A bare `String`/`&str` id is a bug. Function signatures, struct fields, DB row structs, and request
types all use the typed id. This makes "passed a `UserId` where an `AccountId` was expected" a
**compile error**, not a production incident.

```rust
UserId::generate()          // mint a new one
"user_01j9…".parse::<UserId>()?   // validating parse of untrusted input  (-> Result<_, ParseIdError>)
UserId::from_trusted(s)     // wrap a known-good string (DB value, JWT claim we minted) — no validation
user_id.as_str()            // borrow the underlying string (e.g. to bind a query param)
```

## Adding a new id type

One line, alphabetized within its domain group:

```rust
define_id!(pub struct WidgetId => "wgt");
```

- Pick a short, collision-free prefix and keep it in sync with the schema comment on that table's
  `id` column (`-- wgt_{ulid}`).
- **No `///` doc comment on the `define_id!` line** — it breaks the macro (see `id-core/CLAUDE.md`).
  Group types under a plain `//` section comment instead.
- Add a test only when the type carries unusual rules; the macro itself is covered by the generic
  tests already in `lib.rs`.

## How the pieces fit

- **db-queries** decodes columns into these types (`AS "id: UserId"`) and binds them via `.as_str()`.
  Row structs hold typed ids; see `db-queries/CLAUDE.md`.
- **api-gateway** takes typed ids in service/authz signatures. Ids arriving as path/query strings are
  `.parse()`d (→ 400 on malformed input) before use. Ids we minted and carry in a signed JWT are
  re-wrapped with `from_trusted` (no re-validation — the signature already vouches for them).
- **serde**: `#[serde(transparent)]`, so an id serializes/deserializes as the bare string in JSON.
- **sqlx**: `#[sqlx(transparent)]` TEXT, so binds and column overrides just work.

## Trusted vs. validated — pick deliberately

- Crossing a trust boundary (HTTP body/path/query, CLI arg, external webhook)? → `parse()` / `from_str`.
- Value we produced and are round-tripping (DB read, our own JWT claim)? → `from_trusted` (cheaper).

When unsure, validate.
