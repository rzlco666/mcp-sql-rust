# Changelog

## 0.5.2 — 2026-07-14

### P0 — Metadata

- **`describe_table` index `unique`:** no longer hardcodes `false`; preserves uniqueness from MySQL/`SQLite`/`Postgres` index metadata (Postgres via `CREATE UNIQUE INDEX` in `indexdef`).

### P1 — Schema/DDL UX

- **DDL aliases** (`create_table`, `drop_table`, `add_column`, `alter_column`, `drop_column`, `truncate_table`) thin-wrap `schema_mutate`.
- **Tool list gating:** DDL tools (including `schema_mutate`) listed only with `--full-tools` **and** `--allow-ddl`.
- **`alter_column` dialects:** MySQL `MODIFY COLUMN`, Postgres `ALTER … TYPE`, SQLite clear unsupported error.

### P1 — Connect UX

- **TCP preflight ≤500ms** before sqlx pool connect (MySQL/Postgres); launcher `PREFLIGHT_TIMEOUT_MS` default **500** (`MCP_SQL_PREFLIGHT_MS` override).

## 0.5.1 — 2026-07-14

### P0 — Type fidelity

- **MySQL text decode:** `VARCHAR` / `CHAR` / `TEXT` / `ENUM` / `SET` / `JSON` in `execute_sql` serialize as UTF-8 JSON strings (no longer hex). Binary types (`BLOB` / `BINARY` / `VARBINARY`) remain hex.
- Shared MySQL text helpers used by schema tools and execute path (`mysql_decode_text_by_*`).

### P1 — Connect UX

- **`--connect-timeout SECS`** (default **2s**, was fixed 5s) for pool acquire timeout on first lazy connect.

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
