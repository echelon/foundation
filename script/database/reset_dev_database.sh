#!/usr/bin/env bash
# Truncate ALL application tables in the DEVELOPMENT database — and only there.
#
# The database name is hardcoded on purpose: this script takes no arguments and refuses to
# run against anything but foundation_development (so it can never be pointed at test — which
# owns its own lifecycle — let alone production). Schema and migration history survive
# (_sqlx_migrations is excluded); only row data is wiped. Sequences restart; TRUNCATE ...
# CASCADE covers any future FK-linked tables.
set -euo pipefail
export PATH="/opt/homebrew/opt/postgresql@16/bin:$PATH"

DB="foundation_development"
URL="postgres://foundation:foundation@localhost:5432/${DB}"

# Belt and braces: verify the server agrees about which database we're connected to.
actual="$(psql "$URL" -Atc "SELECT current_database()")"
if [[ "$actual" != "$DB" ]]; then
  echo "REFUSING: connected to '$actual', expected '$DB'" >&2
  exit 1
fi

tables="$(psql "$URL" -Atc "
  SELECT string_agg(quote_ident(tablename), ', ')
  FROM pg_tables
  WHERE schemaname = 'public' AND tablename <> '_sqlx_migrations'
")"

if [[ -z "$tables" ]]; then
  echo "no application tables in $DB (run migrations first?)"
  exit 0
fi

# Size sanity check: a dev database holds throwaway rows, and more than $MAX_ROWS in any
# table suggests this ISN'T the throwaway data you think it is. Refuse before touching
# anything — no override flag; investigate (or trim the table yourself) and re-run.
MAX_ROWS=1000
oversized=""
while IFS= read -r tbl; do
  [[ -z "$tbl" ]] && continue
  n="$(psql "$URL" -Atc "SELECT count(*) FROM public.\"${tbl}\"")"
  if (( n > MAX_ROWS )); then
    oversized+="  ${tbl}: ${n} rows"$'\n'
  fi
done < <(psql "$URL" -Atc "SELECT tablename FROM pg_tables WHERE schemaname = 'public' AND tablename <> '_sqlx_migrations'")

if [[ -n "$oversized" ]]; then
  {
    echo "REFUSING to truncate: table(s) in $DB hold more than $MAX_ROWS rows —"
    printf '%s' "$oversized"
    echo "That is a lot for a throwaway dev database. Please make sure this is really the"
    echo "data you want to destroy. Nothing was changed."
  } >&2
  exit 1
fi

psql "$URL" -q -c "TRUNCATE $tables RESTART IDENTITY CASCADE"
count="$(psql "$URL" -Atc "SELECT count(*) FROM pg_tables WHERE schemaname = 'public' AND tablename <> '_sqlx_migrations'")"
echo "truncated $count application tables in $DB (schema + migration history kept)"
