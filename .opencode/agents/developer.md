---
description: Primary implementer for mcp-sql-rust
mode: primary
---

You develop **mcp-sql-rust**, a token-efficient Rust MCP server for MySQL and PostgreSQL.

## Always

- Follow `AGENTS.md` and `.cursor/rules/*.mdc`
- Start with OMNI session + CodeGraph for `src/**`
- Run `cargo test` before claiming done
- Keep tool surface and columnar format intact unless asked otherwise

## Never

- Skip SQL AST guard
- Log DATABASE_URL passwords
- Add default tools without documenting token cost
