# Strut Stack SQL

**Light strut. Extreme load.**

Token-efficient [MCP](https://modelcontextprotocol.io) server for **MySQL**, **PostgreSQL**, and **SQLite** — written in Rust. Part of the [StrutStack](https://github.com/rzlco666/strut-stack) ecosystem; usable standalone with any agent client.

[![Latest release](https://img.shields.io/github/v/release/rzlco666/strut-stack-sql)](https://github.com/rzlco666/strut-stack-sql/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/rzlco666/strut-stack-sql/ci.yml?branch=main)](https://github.com/rzlco666/strut-stack-sql/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Formerly published as `mcp-sql-rust` (compat binary alias still ships in v1.0).

## Mission

- **Token thrift** — default **3 tools**, columnar `{cols,rows,meta}`, row/byte caps
- **Best-practice Rust** — AST SQL guard before pool checkout, lean release binary
- **Multi-engine** — PostgreSQL + MySQL + SQLite in one binary
- **Ship-ready** — one-command install, stdio + Streamable HTTP, Cursor-friendly launcher

## Install (one command)

```bash
curl -fsSL https://raw.githubusercontent.com/rzlco666/strut-stack-sql/main/install.sh | sh
strut-stack-sql --version
```

Installs to `~/.local/bin` (also creates `mcp-sql-rust` alias). Ensure `~/.local/bin` is on your `PATH`.

Other channels (Homebrew, Scoop, winget, Docker, cargo): [docs/INSTALL.md](docs/INSTALL.md).

## Quick start (Cursor)

```json
{
  "mcpServers": {
    "strut-stack-sql": {
      "command": "strut-stack-sql",
      "args": ["--full-tools"],
      "cwd": "${workspaceFolder}"
    }
  }
}
```

Project `.env`:

```env
DATABASE_URL=postgresql://user:pass@127.0.0.1:5432/app
```

Without credentials the server still **starts** (ephemeral `sqlite::memory:`) so MCP initialize succeeds; set `DATABASE_URL` for real queries.

Official workspace launcher (loads nested `apps/api/.env`, never blocks start on dead DB):

```json
{
  "mcpServers": {
    "strut-stack-sql": {
      "command": "node",
      "args": [
        "/absolute/path/to/strut-stack-sql/packaging/cursor-mcp-launcher.mjs",
        "${workspaceFolder}"
      ]
    }
  }
}
```

## Why this shape

| Choice | Detail |
|--------|--------|
| Default tools | `search_objects`, `execute_sql`, `analyze_query_performance` |
| Extra tools | `--full-tools` (+ DDL aliases need `--allow-ddl`) |
| Results | Columnar JSON only by default (token-efficient) |
| Guard | `sqlparser` AST; multi-statement deny; write tiers; complexity limits |
| Timeouts | Client timeout + PG `statement_timeout` / MySQL `max_execution_time` |
| Transports | stdio · `--http 127.0.0.1:8080` (`/mcp`, `/healthz`) |

Measured RSS / cold-start vs Node rivals (same seed query): [docs/BENCHMARKS.md](docs/BENCHMARKS.md). We do **not** invent QPS leaderboards.

## CLI

```bash
strut-stack-sql --version
strut-stack-sql                          # stdio, read-only
strut-stack-sql --allow-writes
strut-stack-sql --full-tools
strut-stack-sql --http 127.0.0.1:8080
```

## Docs

| Doc | Purpose |
|-----|---------|
| [docs/QUICKSTART.md](docs/QUICKSTART.md) | First run |
| [docs/INSTALL.md](docs/INSTALL.md) | All install channels |
| [docs/SECURITY.md](docs/SECURITY.md) | Guard + write tiers |
| [docs/TOOLS.md](docs/TOOLS.md) | Tool schemas |
| [docs/CONFIGURATION.md](docs/CONFIGURATION.md) | .env / TOML / CLI |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Modules |
| [docs/BENCHMARKS.md](docs/BENCHMARKS.md) | Reproducible harness |

## Agent clients

See [AGENTS.md](AGENTS.md) for Cursor / OMNI / CodeGraph conventions.

## License

MIT
