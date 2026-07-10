---
name: mcp-sql-dev-workflow
description: >
  Develop and debug mcp-sql-rust (Rust MCP SQL server). Use when editing src/,
  adding tools, changing SQL guard, config, or transports; or when user asks to
  build, test, or extend the MCP server.
---

# mcp-sql-rust Dev Workflow

## Before any edit

1. `omni_session` status
2. `codegraph_explore "<area>"` for structural tasks
3. Read relevant `docs/*.md` section (not whole tree)

## Change recipes

### Add / change MCP tool

1. Handler in `src/tools/core.rs` or `full.rs`
2. Wire `#[tool]` in `src/server.rs`
3. If default surface: add to `CORE_TOOLS`; else `FULL_TOOLS` only
4. Keep description short; update `docs/TOOLS.md`
5. `cargo test && cargo build`

### Change SQL guard

1. Edit `src/guard/classify.rs` / `mod.rs`
2. Add allow+deny unit tests
3. Update `docs/SECURITY.md`
4. Never execute denied SQL in tests against real DB

### Change result format

1. `src/format/columnar.rs` only
2. Preserve `{cols,rows,meta}` contract
3. Update `docs/TOOLS.md` examples

### Config / credentials

1. `src/config.rs` — resolution order: CLI → `.env` walk → TOML
2. Never put secrets in tool schemas
3. Update `docs/CONFIGURATION.md`

## Verify checklist

```bash
cargo test
cargo build --release
./target/release/mcp-sql-rust --help
```

With DB (optional):

```bash
DATABASE_URL=postgresql://... ./target/release/mcp-sql-rust
```

## After task

- `codegraph sync` if many files changed
- `omni_knowledge` commit for new quirks
- Update docs if public behavior changed
