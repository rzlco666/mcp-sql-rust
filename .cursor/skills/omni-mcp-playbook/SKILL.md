---
name: omni-mcp-playbook
description: >
  Referensi MCP OMNI (user-omni) untuk Cursor. Dipakai saat debug token,
  investigasi besar, atau agent bingung tool mana yang dipanggil.
---

# OMNI MCP Playbook

Server: **`user-omni`**. Call via `CallMcpTool`.

## omni_session

```json
{"server":"user-omni","toolName":"omni_session","arguments":{"action":"status"}}
```

## omni_context

```json
{"server":"user-omni","toolName":"omni_context","arguments":{"file_path":"src/guard/mod.rs"}}
```

## omni_search

```json
{"server":"user-omni","toolName":"omni_search","arguments":{"query":"WriteMode AllowDdl"}}
```

## omni_knowledge

```json
{"server":"user-omni","toolName":"omni_knowledge","arguments":{"action":"query","key":"sql-guard"}}
```

```json
{"server":"user-omni","toolName":"omni_knowledge","arguments":{"action":"commit","key":"sql-guard","value":"EXPLAIN ANALYZE requires --allow-writes"}}
```

## omni_retrieve

When output shows `[OMNI: ... omni_retrieve("hash")]`:

```json
{"server":"user-omni","toolName":"omni_retrieve","arguments":{"hash":"abc123"}}
```

## CLI companions

```bash
omni stats --detail     # savings report
omni stats --today
omni goal set '...'     # north-star for session
omni remember '...'     # durable note
omni exec -- cargo test
```

## Pair with CodeGraph

Structural questions → `codegraph_explore` / `codegraph_node` first. OMNI for session memory + shell distillation.
