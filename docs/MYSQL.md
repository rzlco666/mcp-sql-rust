# MySQL notes — mcp-sql-rust

MySQL support is first-class alongside PostgreSQL. A few behaviors differ from PostgreSQL.

## Schema = database

In MySQL, the `schema` tool parameter is the **database name** (`table_schema` in `information_schema`).

When `schema` is omitted, the server resolves the current database via `SELECT DATABASE()` from the connection URL.

## `describe_table`

- Pass `table` as the bare table name (`fw_users`), not `database.fw_users` in the `table` field (qualified form `db.table` is also accepted and split automatically).
- Backticks in names are stripped (`\`fw_users\`` → `fw_users`).
- Column metadata comes from `information_schema.columns`, with `SHOW FULL COLUMNS` as fallback.

## Columnar JSON / `COUNT(*)`

MySQL may map small integers to boolean in some drivers. mcp-sql-rust decodes **numeric types before boolean** so `COUNT(*)` returns `0` / `1` as numbers, not `false` / `true`.

## `analyze_query_performance`

MySQL `EXPLAIN FORMAT=JSON` uses a different shape than PostgreSQL.

- `total_cost` and `plan_rows` are **best-effort** (aggregated from plan nodes when top-level `cost_info` is missing).
- Sequential scan warnings (`access_type: ALL`) still work.

## Read-only sessions

When not running with `--allow-writes`, MySQL connections get `SET SESSION TRANSACTION READ ONLY` on pool checkout (in addition to the AST guard).

## Integration tests

```bash
MYSQL_DATABASE_URL='mysql://user:pass@host:3306/db' \
  cargo test --test mysql_integration -- --ignored
```
