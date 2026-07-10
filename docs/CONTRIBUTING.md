# Contributing — mcp-sql-rust

## Development

1. Fork / clone https://github.com/rzlco666/mcp-sql-rust
2. Rust 1.85+
3. `cargo test` and `cargo build --release`
4. Optional MySQL integration tests: `MYSQL_DATABASE_URL=mysql://... cargo test --test mysql_integration -- --ignored`

## Commit style

Conventional Commits preferred:

```
feat(guard): block EXPLAIN ANALYZE without --allow-writes
fix(exec): preserve nulls in columnar decode
docs: document multi-source TOML
chore: rename crate to mcp-sql-rust
```

## Pull requests

- Describe **why**, not only what
- Link issues
- Include test plan checklist
- Do not commit `.env`, credentials, `.codegraph/`, or local AI tooling (`.cursor/`, `AGENTS.md`)

## Code review focus

1. SQL guard integrity
2. Token efficiency (tool surface + response shape)
3. No secret leakage
4. Dual-engine correctness (PG + MySQL)

## License

MIT
