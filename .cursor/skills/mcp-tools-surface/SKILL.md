---
name: mcp-tools-surface
description: >
  Design or change MCP tool surface for mcp-sql-rust. Use when adding tools,
  changing schemas, full-tools flag, or optimizing token cost of tool lists.
---

# MCP Tools Surface

## Default (CORE)

1. `search_objects` — progressive schema discovery
2. `execute_sql` — single `sql` or `queries[]` batch
3. `analyze_query_performance` — EXPLAIN summary

## Full (`--full-tools`)

+ `list_sources`, `list_schemas`, `list_tables`, `describe_table`, `list_indexes`

## Design rules

1. Prefer parameters over new tools
2. Description ≤ ~12 words
3. Optional `source` on every DB tool
4. Columnar JSON responses
5. Document in `docs/TOOLS.md`

## Wiring

1. Params struct in `tools/core.rs` or `full.rs`
2. `#[tool]` method on `McpSqlServer`
3. Name in `CORE_TOOLS` or `FULL_TOOLS` in `server.rs`
4. `list_tools` filters by flag

## Token cost reminder

Every tool schema is sent to the LLM each turn. Extra tools = permanent tax.
