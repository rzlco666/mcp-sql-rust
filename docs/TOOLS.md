# MCP Tools — mcp-sql-rust

## Default tools (token-efficient)

### `search_objects`

Progressive schema discovery.

| Param | Type | Notes |
|-------|------|-------|
| `object_type` | `schema` \| `table` \| `column` \| `index` | required |
| `keyword` | string? | case-insensitive filter |
| `schema` | string? | scope |
| `source` | string? | multi-source name |
| `offset` | number? | default 0 |
| `limit` | number? | default 50, max 200 |

### `execute_sql`

Run SQL or concurrent batch.

| Param | Type | Notes |
|-------|------|-------|
| `sql` | string? | single statement |
| `params` | array? | bound values for `?` placeholders (single-query mode) |
| `page_offset` | number? | row offset after guard LIMIT (single-query only; default 0) |
| `page_size` | number? | rows per page (single-query only; default `--max-rows`) |
| `queries` | string[] or object[]? | batch (not both with `sql`) |
| `source` | string? | connection name |

Use `?` for placeholders in the MCP API (PostgreSQL sources auto-rewrite to `$1`, `$2`, … before execution). Advanced PostgreSQL clients may send native `$N` placeholders instead of `?`.

**Single query with params:**

```json
{
  "sql": "SELECT * FROM users WHERE id = ?",
  "params": ["42"]
}
```

**Batch (legacy strings + parameterized objects):**

```json
{
  "queries": [
    "SELECT 1",
    { "sql": "SELECT * FROM users WHERE id = ?", "params": ["42"] }
  ]
}
```

Do not pass top-level `params` together with `queries[]` — put `params` on each batch item instead. `page_offset` / `page_size` are not supported in batch mode.

**Pagination (large result sets):**

```json
{
  "sql": "SELECT id, name FROM users ORDER BY id",
  "page_offset": 0,
  "page_size": 50
}
```

Loop until `meta.has_more` is `false`, incrementing `page_offset` by `page_size` each call. Pagination meta: `page_offset`, `page_size`, `has_more`, `total_fetched`.

**Response (single):**

```json
{
  "ok": true,
  "data": {
    "cols": ["id", "name"],
    "rows": [[1, "alice"]],
    "meta": { "n": 1, "truncated": false, "limit_injected": true }
  }
}
```

`meta.limit_injected` is set when the guard auto-appends `LIMIT` (no user LIMIT). `meta.limit_clamped` is set when a user-specified LIMIT exceeds `--max-rows` and was reduced.

**Response (batch):**

```json
{
  "results": [
    { "ok": true, "data": { "cols": [], "rows": [], "meta": { "n": 0, "truncated": false } } },
    { "ok": false, "error": "SQL guard: DDL blocked; restart with --allow-ddl" }
  ]
}
```

Batch results may arrive out of order relative to the input `queries[]` array (parallel execution). Map results by array index on the client.

### `analyze_query_performance`

Runs `EXPLAIN (FORMAT JSON)` (Postgres), `EXPLAIN FORMAT=JSON` (MySQL), or `EXPLAIN QUERY PLAN` (SQLite). Returns a distilled summary.

Supports optional `params` for `?` placeholders (same rules as `execute_sql`).

```json
{
  "sql": "SELECT * FROM users WHERE id = ?",
  "params": [42]
}
```

On MySQL, `total_cost` and `plan_rows` are best-effort (aggregated from plan nodes when top-level `cost_info` is absent). Sequential scan warnings still apply.

```json
{
  "engine": "postgresql",
  "query": "SELECT ...",
  "total_cost": 12.5,
  "plan_rows": 1000,
  "warnings": ["Sequential scan detected — consider adding an index"],
  "nodes": [{ "node_type": "Seq Scan", "relation": "users", "issues": ["full table scan"] }]
}
```

## Full tools (`--full-tools`)

| Tool | Purpose |
|------|---------|
| `list_sources` | Configured connection names |
| `list_schemas` | Schemas / databases |
| `list_tables` | Tables in schema |
| `describe_table` | Columns + indexes for one table |
| `list_indexes` | Indexes for schema/table |

These wrap the same introspection as `search_objects` — prefer `search_objects` in default mode to save tokens.

## Caps

| Flag | Default | Meaning |
|------|---------|---------|
| `--max-rows` | 100 | Auto-inject `LIMIT` when missing; clamp explicit `LIMIT` above this value |
| `--max-bytes` | 65536 | Truncate columnar payload |
| `--query-timeout` | 10s | Per-query timeout |
| `--batch-concurrency` | 8 | Max parallel batch queries |
| `--pool-size` | 10 | sqlx pool per source |
