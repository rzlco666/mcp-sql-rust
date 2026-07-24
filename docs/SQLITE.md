# SQLite — strut-stack-sql

SQLite is supported as a third engine alongside PostgreSQL and MySQL.

## Connection URLs

| URL | Use case |
|-----|----------|
| `sqlite::memory:` | Ephemeral in-memory (tests, scratch) |
| `sqlite://./path/to/app.db` | File database (relative) |
| `sqlite:/absolute/path/to/app.db` | File database (absolute) |

Environment variables:

```env
DATABASE_URL=sqlite::memory:
SQLITE_URL=sqlite://./data/app.db
```

TOML:

```toml
[[sources]]
name = "local"
url = "sqlite://./app.db"
engine = "sqlite"
```

## Read-only mode

Default **read-only** appends `?mode=ro` when connecting. Writes need `--allow-writes`; DDL needs `--allow-ddl`.

## Placeholders

SQLite uses `?` placeholders. PostgreSQL `$N` placeholders are rejected on SQLite sources.

## Schema tools (PRAGMA-based)

| Tool / path | Implementation |
|-------------|----------------|
| `list_schemas` | `PRAGMA database_list` |
| `list_tables` | `sqlite_master` (default `main`) |
| `describe_table` | `PRAGMA table_info('table')` + `PRAGMA index_list('table')` |
| `list_indexes` | `PRAGMA index_list` / `index_info` |
| `search_objects` | Same via `schema.rs` |

**Note:** SQLite `PRAGMA` table names are embedded as quoted literals (not `?` bind params) — required by SQLite semantics.

Qualified names `attached.table` work when the attached DB exists.

## EXPLAIN

`analyze_query_performance` runs `EXPLAIN QUERY PLAN` and maps `detail` rows into `ExplainSummary`. Supports optional `params`.

No JSON plan like PostgreSQL/MySQL.

## AST guard

Uses `SQLiteDialect` in `sqlparser`. Same write tiers as other engines.

## Limitations

- No server session diagnostics (`SHOW PROCESSLIST` equivalent)
- Attached-database coverage is best-effort
- Columnar JSON only (no streaming)
- Compose dev stack focuses on PG+MySQL; SQLite needs no Docker

## Tests

```bash
cargo test --test sqlite_integration
```

See [CONFIGURATION.md](CONFIGURATION.md) and [SECURITY.md](SECURITY.md).
