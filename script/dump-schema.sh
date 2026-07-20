#!/usr/bin/env bash
# Regenerate the materialized schema rollup from a database (default: foundation_development).
set -euo pipefail
export PATH="/opt/homebrew/opt/postgresql@16/bin:$PATH"
DB="${1:-foundation_development}"
here="$(cd "$(dirname "$0")" && pwd)"
{
  echo "-- ============================================================================"
  echo "-- MATERIALIZED SCHEMA ROLLUP — generated, do not edit by hand."
  echo "-- Regenerate: script/dump-schema.sh"
  echo "-- ============================================================================"
  echo
  pg_dump --schema-only --no-owner --no-privileges --no-comments --exclude-table=_sqlx_migrations -d "$DB"
} > "$here/../crates/db/schema.sql"
echo "wrote crates/db/schema.sql from $DB"
