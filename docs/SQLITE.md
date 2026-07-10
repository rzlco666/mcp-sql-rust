# SQLite — mcp-sql-rust

SQLite is supported as a third engine alongside PostgreSQL and MySQL (v0.2.0+).

## Connection URLs

| URL | Use case |
|-----|----------|
| `sqlite::memory:` | Ephemeral in-memory database (tests, scratch) |
| `sqlite://./path/to/app.db` | File database (relative path) |
| `sqlite:/absolute/path/to/app.db` | File database (absolute path) |
| `sqlite:file:./data.db` | Alternate file URI form |

Environment variables:

```env
DATABASE_URL=sqlite::memory:
# or
SQLITE_URL=sqlite://./data/app.db
```

TOML source hint:

```toml
[[sources]]
name = "local"
url = "sqlite://./app.db"
engine = "sqlite"
```

## Read-only mode

Default **read-only** appends `?mode=ro` to the SQLite URL when connecting (sqlx-supported). Writes require `--allow-writes`; DDL requires `--allow-ddl`.

## Placeholders

SQLite uses `?` placeholders (same as MySQL). PostgreSQL `$N` placeholders are rejected on SQLite sources.

## Schema tools

| Tool / path | SQLite implementation |
|-------------|----------------------|
| `list_schemas` | `PRAGMA database_list` (`main` + attached DBs) |
| `list_tables` | `sqlite_master` (default schema `main`) |
| `describe_table` | `PRAGMA table_info(?)` + `PRAGMA index_list` |
| `search_objects` | Same paths via `schema.rs` |

Qualified names `attached.table` are supported when the attached database exists.

## EXPLAIN

`analyze_query_performance` runs `EXPLAIN QUERY PLAN` and maps text `detail` rows into the shared `ExplainSummary` format. There is no JSON plan like PostgreSQL/MySQL.

## Limitations (v0.2.0)

- No `SHOW PROCESSLIST` or server session diagnostics
- Attached-database coverage is best-effort (default `main`)
- Result streaming is not implemented (columnar JSON only)

See also [CONFIGURATION.md](CONFIGURATION.md) and [SECURITY.md](SECURITY.md).
