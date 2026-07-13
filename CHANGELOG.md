# Changelog

## 0.5.0 — 2026-07-13

### P0 — Blockers

- **Type fidelity:** MySQL `DECIMAL` / `BIGINT UNSIGNED` / aggregates (`SUM`, `ROUND`, `information_schema`) serialize as JSON numbers via `bigdecimal` decoding.
- **Lazy connect (default):** MCP starts without blocking on DB; 5s connect timeout per tool call. `--eager-connect` restores startup ping.
- **Schema isolation:** `search_objects` / `list_tables` default to current database; use `schema: "*"` to search all schemas on MySQL.
- **Cursor setup:** Official `packaging/cursor-mcp-launcher.mjs` with TCP preflight, `--workspace`, and [docs/CURSOR.md](docs/CURSOR.md).

### P1 — Parity

- **`schema_mutate`** tool (behind `--full-tools` + `--allow-ddl`) with `confirm: true` for destructive ops.
- **`list_foreign_keys`** tool for MySQL, PostgreSQL, SQLite.
- **`describe_table`** enriched: `key`, `extra`, `comment`, `default` (MySQL via `SHOW FULL COLUMNS`).
- **`list_tables`** pagination (`offset`, `limit`, `has_more` meta).
- **`execute_sql` `format`:** `auto` | `columnar` | `rows` (+ `MCP_SQL_FORMAT` env).
- **Security:** URL password redaction in connect errors; `MCP_SQL_LOG=audit` SQL audit trail.

### P2 — Polish

- Benchmark script: `scripts/benchmark/run-mysql-vs-mcp-mysql.sh`
- [docs/POSTGRES.md](docs/POSTGRES.md), updated [docs/MYSQL.md](docs/MYSQL.md), [docs/CURSOR.md](docs/CURSOR.md)

## 0.4.0

- HTTP transport, server-side pagination, MCP Registry packaging.
