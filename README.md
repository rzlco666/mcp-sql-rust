# mcp-sql-rust

**Token-efficient [MCP](https://modelcontextprotocol.io) server for MySQL, PostgreSQL, and SQLite** — written in Rust.

[![Latest release](https://img.shields.io/github/v/release/rzlco666/mcp-sql-rust)](https://github.com/rzlco666/mcp-sql-rust/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/rzlco666/mcp-sql-rust/ci.yml?branch=main)](https://github.com/rzlco666/mcp-sql-rust/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)
[![GHCR](https://img.shields.io/badge/ghcr.io-container-blue)](https://github.com/rzlco666/mcp-sql-rust/pkgs/container/mcp-sql-rust)

Registry: `io.github.rzlco666/mcp-sql-rust` · Repo: https://github.com/rzlco666/mcp-sql-rust

## Try in 60 seconds

```bash
git clone https://github.com/rzlco666/mcp-sql-rust.git && cd mcp-sql-rust
docker compose up -d
cp .env.example .env
curl -fsSL https://raw.githubusercontent.com/rzlco666/mcp-sql-rust/main/install.sh | sh
```

Or **Reopen in Container** (`.devcontainer/`) for Postgres + MySQL + Rust toolchain pre-wired.

Full walkthrough: [docs/QUICKSTART.md](docs/QUICKSTART.md)

## Why Rust?

| Server | Idle RSS | Cold start | p50 query* | tools/list |
|--------|----------|------------|------------|------------|
| **mcp-sql-rust** | **~10 MB** | **~13 ms** | ~0.9 ms | ~2.5 KB |
| server-postgres (Node) | ~100 MB | ~560 ms | ~1.5 ms | ~0.1 KB |
| postgres-mysql-mcp-server | ~101 MB | ~550 ms | ~0.3 ms | ~2 KB |

\*Same PostgreSQL seed, `SELECT … FROM demo.users WHERE id = 42`. Full methodology: [docs/BENCHMARKS.md](docs/BENCHMARKS.md)

**mcp-sql-rust** ships a **3-tool default surface**, columnar JSON, AST SQL guard, and dual-engine pools — without a Node/Python runtime.

## Features

| Feature | Detail |
|---------|--------|
| Engines | PostgreSQL + MySQL + SQLite (`sqlx`) |
| Zero-config | Walks cwd→parents for `.env` |
| Token-efficient | 3 core tools; columnar `{cols,rows,meta}` |
| SQL guard | `sqlparser` AST deny **before** pool checkout |
| Write tiers | read-only → `--allow-writes` → `--allow-ddl` |
| Pagination | `page_offset` / `page_size` on `execute_sql` |
| Concurrency | Parallel tools + `queries[]` batch |
| Transports | stdio + Streamable HTTP (`--http`) |
| Registry | MCPB bundles on GitHub Releases |

## Install

```bash
# curl (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/rzlco666/mcp-sql-rust/main/install.sh | sh

# Homebrew
brew install rzlco666/tap/mcp-sql-rust

# Docker
docker run --rm -i -e DATABASE_URL=... ghcr.io/rzlco666/mcp-sql-rust:latest
```

All channels (Scoop, winget, cargo-binstall, manual archives + SHA256): **[docs/INSTALL.md](docs/INSTALL.md)**

## Quick start (Cursor)

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
DATABASE_URL=postgresql://demo:demo@localhost:5433/demo
```

## CLI

```bash
mcp-sql-rust --version
mcp-sql-rust                          # stdio, read-only
mcp-sql-rust --allow-writes
mcp-sql-rust --full-tools
mcp-sql-rust --http 127.0.0.1:8080
```

HTTP serves MCP at `/mcp`; health check at `/healthz`.

## Default tools

| Tool | Role |
|------|------|
| `search_objects` | Progressive schema discovery |
| `execute_sql` | SQL + batch + pagination + `params` |
| `analyze_query_performance` | Distilled EXPLAIN |

See [docs/TOOLS.md](docs/TOOLS.md).

## Documentation

| Doc | Topic |
|-----|-------|
| [docs/QUICKSTART.md](docs/QUICKSTART.md) | 60-second setup + devcontainer |
| [docs/INSTALL.md](docs/INSTALL.md) | All install channels |
| [docs/BENCHMARKS.md](docs/BENCHMARKS.md) | Performance vs rivals |
| [docs/TEST_REPORT.md](docs/TEST_REPORT.md) | v0.4.0 test matrix & audit |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Modules & data flow |
| [docs/TOOLS.md](docs/TOOLS.md) | Tool schemas |
| [docs/SECURITY.md](docs/SECURITY.md) | Guard & write tiers |
| [docs/CONFIGURATION.md](docs/CONFIGURATION.md) | .env / TOML / CLI |
| [docs/MYSQL.md](docs/MYSQL.md) | MySQL notes |
| [docs/SQLITE.md](docs/SQLITE.md) | SQLite URLs & limits |
| [docs/MCP_REGISTRY.md](docs/MCP_REGISTRY.md) | Registry publish |
| [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) | PRs & development |

## Security

Default is **read-only**. Destructive SQL is blocked at the AST layer with **zero DB round-trip** on deny. Details: [docs/SECURITY.md](docs/SECURITY.md).

## License

MIT
