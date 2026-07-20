# db-queries — the ONLY SQL in the codebase

Every SQL statement in Foundation lives in this crate. No other crate writes SQL; they call these
functions. This keeps the query surface auditable, compile-checked, and typed end to end.

## Layout: one query per file, grouped by table

```
src/
  lib.rs                       # declares the table modules + re-exports row structs
  <table>/
    mod.rs                     # the row struct(s) + `mod`/`pub use` for each query file
    <verb>_<noun>.rs           # exactly ONE public query fn
```

- File name == fn name, e.g. `accounts/create_account.rs` exports `create_account`; `users/find_user_by_email.rs`
  exports `find_user_by_email`. Verb-first, unabbreviated.
- The **row struct** for a table (`Account`, `User`, `Session`, …) is declared in that table's `mod.rs`
  and re-exported at the crate root (`db_queries::User`). Input structs (`NewUser`, `NewSession`,
  `NewInvitation`, …) live next to the query that consumes them and are re-exported from `mod.rs`.
- Add a new query → new file + two lines in `mod.rs` (`mod x;` / `pub use x::x;`). Never grow a
  grab-bag module.

## Rule 1 — ALWAYS use the compile-checked macros. NEVER build query strings.

Use `sqlx::query!`, `query_as!`, or `query_scalar!`. These validate the SQL against the live schema
at compile time (via the `.sqlx/` offline cache) and check that every column and bind param type
lines up. Runtime string queries (`sqlx::query(&format!(...))`) are **forbidden** — they move errors
from `cargo build` to production.

The **only** sanctioned reason to drop the macros is a genuinely dynamic statement the macro can't
express — chiefly a `WHERE col IN (…)` over a runtime-length list, or a dynamically-assembled filter
set. For those, use `sqlx::QueryBuilder` (still no `format!`-ed SQL). If you reach for it, add a
one-line comment saying why the macro didn't fit.

- `query_as!` maps columns to struct fields **by position** — the `SELECT` / `RETURNING` column
  order MUST match the struct's field order. Keep them adjacent and commented.
- Prefer explicit column lists over `SELECT *` so the mapping is stable when the table grows.

## Rule 2 — strongly-typed ids, always (see the `ids` crate)

Ids are never bare `String`. Columns decode into typed newtypes and params bind via `.as_str()`:

```rust
sqlx::query_as!(
  User,
  r#"SELECT id AS "id: UserId", email, created_at
     FROM users WHERE id = $1 AND deleted_at IS NULL"#,
  id.as_str(),                 // id: &UserId
)
```

- Column override for a typed id: `col AS "col: TypeId"`. For a **nullable** typed id use the `?`
  form: `col AS "col?: TypeId"` → `Option<TypeId>`.
- You may fully-qualify inside the override string (`AS "id: ids::UserId"`) — the macro needs the
  path in scope; both `UserId` (imported) and `ids::UserId` work.
- Query fns take id **references** (`&UserId`, `&AccountId`) and bind `.as_str()`.

## Rule 3 — executor-generic; transactions compose

Every query fn is generic over the executor so it runs against a pool *or* an open transaction:

```rust
pub async fn create_account<'e, E: PgExecutor<'e>>(exec: E, name: Option<&str>) -> sqlx::Result<Account>
```

Multi-statement flows (signup: user + account + membership + session) `begin()` a transaction in the
**caller** (the api-gateway service layer) and thread `&mut *tx` into each query. This crate exposes
single-statement primitives; it does not own transaction boundaries.

## Rule 4 — return `sqlx::Result<T>`; no domain errors here

Queries return `sqlx::Result<…>`. Mapping DB errors to HTTP/domain errors is the caller's job. Shapes:
`fetch_one` → `T`, `fetch_optional` → `Option<T>`, `fetch_all` → `Vec<T>`. Mutations that report
whether they hit a row return `bool` (`execute(...).rows_affected() > 0`) or the raw `u64` count when
the caller needs the number (e.g. "revoke N other sessions").

## Rule 5 — integrity lives in the schema, enforced by CHECKs and partial unique indexes

No FKs/triggers (see the schema). CHECK constraints (`role IN ('owner','admin','member')`) and partial
unique indexes (one `owner` per account) are cheap and are the last line of defense — a query that
violates one returns a `sqlx::Error` the caller surfaces as a 409/400. Tests assert these rejections.

## The `.sqlx/` offline cache — keep it in sync

The macros validate against `.sqlx/` (checked into git) so CI and teammates build without a database.
After adding or changing any query, regenerate it:

```
DATABASE_URL="postgres://foundation@localhost/foundation_development" cargo sqlx prepare --workspace
```

Commit the resulting `.sqlx/` changes alongside the query. A stale cache = build failures for everyone.

## Testing

Integration tests (`tests/`) run against `foundation_test` via `db::test_fixtures::create_test_pool()`,
which **hard-guards** the URL ends in `_test` (never `_development`/`_production`). Each test runs in a
transaction that is dropped (rolled back) at the end, so the DB stays clean and tests are parallel-safe.
Use `id_core::mint("…")` for unique throwaway strings (e.g. unique emails).

## Postgres / Rust gotchas

- `inet` params: binding `Option<&str>` to an `inet` column trips macro type-inference. Cast the
  param, not the column: `VALUES (…, ($4::text)::inet, …)`. This avoids pulling in the sqlx
  `ipnetwork` feature just to store a request IP.
- `now()` for server-set timestamps; never send a client clock for `created_at`/`revoked_at`.
- Exposed columns only in row structs — never select `sid` (the internal bigint is `ORDER BY` fuel,
  not part of the API). Order lists by `sid DESC` for newest-first.
