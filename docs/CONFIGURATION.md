# Configuration — mcp-sql-rust

## Resolution order

1. CLI `--url` / `--url-env`
2. Walk cwd → parents for `.env`, then:
   - `DATABASE_URL`
   - `POSTGRES_URL` / `MYSQL_URL`
   - `POSTGRES_*` / `PG*` parts
   - `MYSQL_*` parts
3. `--config mcp-sql-rust.toml` multi-source
4. Error listing searched paths

## `.env` examples

```env
DATABASE_URL=postgresql://user:pass@localhost:5432/app
```

```env
DATABASE_URL=mysql://user:pass@localhost:3306/app
```

```env
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_USER=app
POSTGRES_PASSWORD=secret
POSTGRES_DB=app
```

## Multi-source TOML

See [`mcp-sql-rust.example.toml`](../mcp-sql-rust.example.toml):

```toml
default_source = "app"

[[sources]]
name = "app"
url_env = "DATABASE_URL"

[[sources]]
name = "analytics"
url = "postgresql://readonly:secret@localhost:5432/analytics"
engine = "postgres"
```

Pass `"source": "analytics"` in tool calls.

## CLI reference

```bash
mcp-sql-rust                          # stdio, .env, read-only
mcp-sql-rust --allow-writes
mcp-sql-rust --allow-ddl
mcp-sql-rust --full-tools
mcp-sql-rust --http 127.0.0.1:8080
mcp-sql-rust --config ./mcp-sql-rust.toml
mcp-sql-rust --url "$DATABASE_URL"
mcp-sql-rust --url-env DATABASE_URL
mcp-sql-rust --max-rows 50 --max-bytes 32768
mcp-sql-rust --pool-size 16 --query-timeout 15
mcp-sql-rust --batch-concurrency 4 --fail-fast
```

## Cursor MCP config

```json
{
  "mcpServers": {
    "sql": {
      "command": "mcp-sql-rust",
      "args": []
    }
  }
}
```

With writes (local only):

```json
{
  "mcpServers": {
    "sql": {
      "command": "mcp-sql-rust",
      "args": ["--allow-writes"]
    }
  }
}
```

Put secrets in project `.env`, not in `args`.

## Logging

```bash
MCP_SQL_LOG=debug mcp-sql-rust
```

Logs go to **stderr** (stdout is MCP stdio).
