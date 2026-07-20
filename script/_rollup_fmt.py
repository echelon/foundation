#!/usr/bin/env python3
"""Format a `table<TAB>column<TAB>type` stream (from psql) into docs/schema-rollup.md."""
import collections
import sys

rows = collections.OrderedDict()
for line in sys.stdin:
    line = line.rstrip("\n")
    if not line:
        continue
    t, c, ty = line.split("\t")
    rows.setdefault(t, []).append((c, ty))

out = [
    "# Schema rollup (materialized, for documentation)",
    "",
    "Generated from the database after migrations. Human-readable companion to",
    "`crates/db/schema.sql` (the full pg_dump). Regenerate with `script/schema-rollup.sh`.",
    "As migrations layer on top, regenerate and diff to evaluate the net schema.",
    "",
    f"**{len(rows)} tables.** Every table opens with `sid bigint` (identity, ORDER BY only) + `id text` PK.",
    "Enum-ish columns are `text` + CHECK; no foreign keys or triggers (integrity in code).",
    "",
]
for t, cols in rows.items():
    out.append(f"### {t}\n")
    out.append("| column | type |")
    out.append("| ------ | ---- |")
    out += [f"| {c} | {ty} |" for c, ty in cols]
    out.append("")
sys.stdout.write("\n".join(out))
