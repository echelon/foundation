#!/usr/bin/env bash
# Regenerate docs/schema-rollup.md (human-readable column rollup) from a database.
set -euo pipefail
export PATH="/opt/homebrew/opt/postgresql@16/bin:$PATH"
DB="${1:-foundation_development}"
here="$(cd "$(dirname "$0")" && pwd)"
psql -d "$DB" -tAF $'\t' -c "select c.table_name, c.column_name, c.data_type || case when c.is_nullable='NO' then ' NOT NULL' else '' end from information_schema.columns c join pg_tables t on t.tablename=c.table_name and t.schemaname='public' where c.table_schema='public' and c.table_name not like '%_default' order by c.table_name, c.ordinal_position" \
| python3 "$here/_rollup_fmt.py" > "$here/../docs/schema-rollup.md"
echo "wrote docs/schema-rollup.md from $DB"
