# mcp-sql-rust

**Token-efficient [MCP](https://modelcontextprotocol.io) server for MySQL and PostgreSQL**, written in Rust.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)

Repo: https://github.com/rzlco666/mcp-sql-rust

## Why

Existing MCP SQL servers often:

1. Fail to find credentials even when `.env` exists  
2. Run single-threaded / serial queries  
3. Waste tokens dumping schemas and row-object JSON  
4. Rely on weak keyword filters for safety  

**mcp-sql-rust** fixes those with Tokio + sqlx pools, `.env` auto-discovery, a 3-tool default surface, columnar results, and `sqlparser` AST guards.

## Features

| Feature | Detail |
|---------|--------|
| Dual engine | PostgreSQL + MySQL (`sqlx`) |
| Zero-config | Walks cwd→parents for `.env` |
| Token-efficient | 3 core tools; columnar JSON |
| Smart SQL guard | AST deny before DB hit |
| Write tiers | read-only → `--allow-writes` → `--allow-ddl` |
| Concurrency | Parallel tools + `queries[]` batch |
| EXPLAIN helper | `analyze_query_performance` |
| Multi-source | Optional TOML |
| Transports | stdio + Streamable HTTP |

## Install

### From GitHub Releases (recommended)

Download a pre-built binary for your platform from the [latest release](https://github.com/rzlco666/mcp-sql-rust/releases/latest):

```bash
# Linux x86_64 example
curl -LO https://github.com/rzlco666/mcp-sql-rust/releases/latest/download/mcp-sql-rust-x86_64-unknown-linux-gnu.tar.gz
tar -xzf mcp-sql-rust-x86_64-unknown-linux-gnu.tar.gz
sudo mv mcp-sql-rust /usr/local/bin/
```

macOS: download the `mcp-sql-rust-*-apple-darwin` binary from Releases and place it on your `PATH`.

### Build from source

```bash
git clone https://github.com/rzlco666/mcp-sql-rust.git
cd mcp-sql-rust
cargo install --path .
```

Binary: `mcp-sql-rust`

## Quick start (Cursor)

`.cursor/mcp.json` / user MCP config:

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

Project `.env`:

```env
DATABASE_URL=postgresql://user:pass@localhost:5432/mydb
```

## CLI

```bash
mcp-sql-rust                          # stdio, .env, read-only
mcp-sql-rust --allow-writes
mcp-sql-rust --allow-ddl
mcp-sql-rust --full-tools
mcp-sql-rust --http 127.0.0.1:8080
mcp-sql-rust --config ./mcp-sql-rust.toml
```

### HTTP mode & production deploy

Streamable HTTP serves MCP at `/mcp`. A health check endpoint is available at `/healthz` (pings all configured database pools; returns `503` if any pool fails).

```bash
# Local
mcp-sql-rust --http 127.0.0.1:8080

# Health check
curl -s http://127.0.0.1:8080/healthz
```

For production, bind to localhost and terminate TLS at a reverse proxy (Caddy, nginx, etc.):

```
https://mcp.example.com/mcp   →  http://127.0.0.1:8080/mcp
https://mcp.example.com/healthz → http://127.0.0.1:8080/healthz
```

Do not expose the HTTP server directly to the internet without TLS and access controls.

## Default tools

| Tool | Role |
|------|------|
| `search_objects` | Search schemas/tables/columns/indexes |
| `execute_sql` | SQL or concurrent `queries[]` |
| `analyze_query_performance` | Distilled EXPLAIN |

See [docs/TOOLS.md](docs/TOOLS.md).

## Documentation

| Doc | Topic |
|-----|-------|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Modules & data flow |
| [docs/TOOLS.md](docs/TOOLS.md) | Tool schemas |
| [docs/SECURITY.md](docs/SECURITY.md) | Guard & write tiers |
| [docs/CONFIGURATION.md](docs/CONFIGURATION.md) | .env / TOML / CLI |
| [docs/MYSQL.md](docs/MYSQL.md) | MySQL-specific behavior |
| [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) | PRs & development |

## Security

Default is **read-only**. Destructive SQL is blocked at the AST layer with **zero DB round-trip**. Details: [docs/SECURITY.md](docs/SECURITY.md).

## License

MIT
