# id-core — the id *machinery* (no concrete types)

This crate holds the mechanism for strongly-typed ids and **declares zero concrete id types**. The
`{Entity}Id` newtypes live in the `ids` crate (which depends on this one). Almost everything in the
workspace depends on `ids`, not `id-core` directly — depend on `id-core` only when you need `mint`,
`validate`, or the `define_id!` macro itself.

## What lives here

- `mint(prefix) -> String` — `{prefix}_{26-char lowercase Crockford ULID}`. Time-ordered + globally
  unique. This is the ONLY place ids are minted.
- `validate(s, prefix) -> Result<(), ParseIdError>` — checks the `{prefix}_` and a 26-char ULID body.
- `ParseIdError` — `WrongPrefix` / `BadUlid`, returned by every `FromStr`.
- `define_id!` — the macro that stamps out a concrete newtype (see below).

## The id format

`user_01j9z4k7m8n3p0q9r8s7t6v5w4` — `{prefix}_{ulid}`, the ulid lowercased to stay case-stable in URLs
and TEXT columns. Prefixes are short, unabbreviated-enough to be obvious (`acct`, `user`, `sess`,
`inv`, `sso`, …). The full prefix registry is the set of `define_id!` calls in the `ids` crate.

## `define_id!` — what the generated type gives you

`define_id!(pub struct UserId => "user");` produces a `#[sqlx(transparent)] #[serde(transparent)]`
newtype over `String` with:

- `UserId::generate()` — fresh id.
- `UserId::from_str(s)` / `s.parse()` — **validating** parse for untrusted input (returns `ParseIdError`).
- `UserId::from_trusted(s)` — skip validation for strings we ourselves produced (a DB value, a JWT
  claim we minted). Use this on hot paths where the value is already known-good; never on user input.
- `as_str()` / `into_string()`, `Display`, `Debug`, `AsRef<str>`, and the full `Ord`/`Hash`/`Eq` set.
- `sqlx::Type` (TEXT) so it drops straight into `query_as!` overrides (`AS "id: UserId"`) and binds.

### Gotcha: no `///` doc comment on a `define_id!` invocation line

A `///`/`//` comment on the same logical line as a `define_id!(...)` call swallows the rest of the
line — including the closing `)` — and breaks the macro. Document the type on its own line above the
call, or in the module, not inside the macro args.

The macro requires `serde` and `sqlx` to be dependencies of the **invoking** crate (they are, in `ids`).
