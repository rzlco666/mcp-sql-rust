---
name: sql-guard-safety
description: >
  Modify or review the SQL AST guard and write tiers in mcp-sql-rust. Use when
  changing classify/validate, read-only policy, EXPLAIN rules, or security docs.
---

# SQL Guard Safety Skill

## Files

- `src/guard/classify.rs` — statement → Read/Dml/Ddl/Txn/Other
- `src/guard/mod.rs` — parse, enforce, LIMIT inject
- `src/db/exec.rs` — calls guard before execute
- `docs/SECURITY.md` — policy documentation

## Checklist for policy changes

- [ ] Dialect-aware parse (PG + MySQL)
- [ ] Multi-statement rejected
- [ ] Tier matrix tests (RO / writes / ddl)
- [ ] EXPLAIN ANALYZE gated
- [ ] Deny path does not touch DB
- [ ] Docs updated

## Test snippets

```bash
cargo test -- guard
cargo test -- --nocapture blocks_
```

## Never

- Keyword-only filters replacing AST
- Allowing `BEGIN`/`COMMIT` as escape hatch in read-only
- Returning raw connection errors that include passwords
