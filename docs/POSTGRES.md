# PostgreSQL — mcp-sql-rust notes

## Connection

```env
DATABASE_URL=postgresql://user:pass@localhost:5432/app
```

## Quirks

- Read-only mode sets `default_transaction_read_only = on` on each pool connection.
- Placeholders: use `?` in MCP tools; rewritten to `$1`, `$2`, … before execution.
- Schema default for `search_objects` / `list_tables`: `public` when omitted.
- `EXPLAIN (FORMAT JSON)` used by `analyze_query_performance`.
- `numeric` / `DECIMAL` columns serialize as JSON numbers (or strings for extreme precision).

## Docker dev

```bash
docker compose up -d postgres
DATABASE_URL=postgresql://demo:demo@localhost:5433/demo cargo test --test http_integration -- --ignored
```

See [CONFIGURATION.md](CONFIGURATION.md) and [TOOLS.md](TOOLS.md).
