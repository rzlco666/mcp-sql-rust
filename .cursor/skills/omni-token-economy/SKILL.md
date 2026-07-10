---
name: omni-token-economy
description: >
  WAJIB di awal setiap task coding/debug di mcp-sql-rust. Ritual OMNI session +
  omni_knowledge keys. Tool ladder canonical di omni-agent-protocol rule.
---

# OMNI Token Economy — mcp-sql-rust

**Tool ladder:** `.cursor/rules/omni-agent-protocol.mdc`  
**Playbook:** skill `omni-mcp-playbook`  
**Filter:** `.omni/filters/mcp-sql-rust.toml`

## Ritual awal

1. `omni_session` → `{"action":"status"}`
2. `omni_knowledge` → `{"action":"query","key":"<topic>"}`

## Ritual akhir

- `omni_knowledge` commit for new facts
- Large investigation → `omni_budget` once
- Optional CLI: `omni stats --detail`

## omni_knowledge keys

| Key | Content |
|-----|---------|
| `sql-guard` | AST tiers, EXPLAIN ANALYZE, multi-stmt deny |
| `columnar-format` | cols/rows/meta contract |
| `rmcp-tools` | CORE vs FULL tool lists |
| `sqlx-dual` | EnginePool PG/MySQL patterns |
| `token-efficiency` | caps, no schema dump, short descriptions |
| `config-dotenv` | .env walk + TOML multi-source |

## Why this matters

OMNI historically cuts ~90%+ of noisy shell/rg/diff tokens (`omni stats --detail`). Prefer distilled signal over raw dumps so long sessions stay within context.
